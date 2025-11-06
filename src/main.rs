use std::env;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{PathBuf};
use std::process::{self, Stdio};

#[derive(Debug, Clone)]
struct Redirect {
    fd: i32,
    target: String,
}

fn tokenize(input: &str) -> Result<(Vec<String>, Option<Redirect>), String> {
    let mut current_token = String::new();
    let mut tokens: Vec<String> = Vec::new();
    let mut input_chars = input.chars().peekable();
    let mut is_in_single_quotes = false;
    let mut is_in_double_quotes = false;
    while let Some(ch) = input_chars.next() {
        match ch {
            '\\' if !is_in_single_quotes => {
                if let Some(&next_char) = input_chars.peek() {
                    if is_in_double_quotes {
                        match next_char {
                            '"' | '$' | '\\' | '`' | '\n' => {
                                current_token.push(next_char);
                                input_chars.next();
                            }
                            _ => {
                                current_token.push('\\');
                                current_token.push(next_char);
                                input_chars.next();
                            }
                        }
                    } else {
                        current_token.push(next_char);
                        input_chars.next();
                    }
                } else {
                    current_token.push('\\');
                }
            }
            '"' => {
                if is_in_single_quotes {
                    current_token.push(ch);
                } else {
                    is_in_double_quotes = !is_in_double_quotes;
                }
            }
            '\'' => {
                if is_in_double_quotes {
                    current_token.push(ch);
                } else {
                    is_in_single_quotes = !is_in_single_quotes;
                }
            }
            ch if ch.is_whitespace() => {
                if is_in_single_quotes || is_in_double_quotes {
                    current_token.push(ch);
                } else if !current_token.is_empty() {
                    tokens.push(current_token.clone());
                    current_token.clear();
                }
            }
            _ => {
                current_token.push(ch);
            }
        }
    }
    if !current_token.is_empty() {
        tokens.push(current_token);
    }
    let redirect = if tokens.len() >= 2 {
        let op_token = tokens[tokens.len() - 2].as_str();
        let fd = if op_token == ">" {
            Some(1)
        } else if op_token.ends_with('>') {
            let fd_str = &op_token[..op_token.len() - 1];
            if fd_str.is_empty() {
                Some(1)
            } else {
                Some(
                    fd_str
                        .parse::<i32>()
                        .map_err(|_| format!("invalid file descriptor: {}", fd_str))?,
                )
            }
        } else {
            None
        };

        if let Some(fd) = fd {
            let filename = tokens
                .pop()
                .ok_or_else(|| "missing file name for redirect".to_string())?;
            tokens.pop();
            Some(Redirect { fd, target: filename })
        } else {
            None
        }
    } else {
        None
    };

    Ok((tokens, redirect))
}

fn get_write_output(redirect_filename: &str) -> io::Result<File> {
    OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(redirect_filename)
}

enum OutputSink<'a> {
    Stdout(io::StdoutLock<'a>),
    File(File),
}

impl<'a> Write for OutputSink<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            OutputSink::Stdout(handle) => handle.write(buf),
            OutputSink::File(file) => file.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            OutputSink::Stdout(handle) => handle.flush(),
            OutputSink::File(file) => file.flush(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum BuiltinFlow {
    Continue,
    Exit(i32),
}

type BuiltinFn = fn(&mut Shell, &[String], &mut dyn Write) -> io::Result<BuiltinFlow>;

struct Shell {
    builtins: std::collections::HashMap<&'static str, BuiltinFn>,
}

impl Shell {
    fn new() -> Self {
        use std::collections::HashMap;

        let mut builtins: HashMap<&'static str, BuiltinFn> = HashMap::new();
        builtins.insert("exit", Shell::builtin_exit);
        builtins.insert("echo", Shell::builtin_echo);
        builtins.insert("type", Shell::builtin_type);
        builtins.insert("pwd", Shell::builtin_pwd);
        builtins.insert("cd", Shell::builtin_cd);

        Shell { builtins }
    }

    fn run(&mut self) -> io::Result<()> {
        loop {
            print!("$ ");
            io::stdout().flush()?;

            let mut command = String::new();
            if io::stdin().read_line(&mut command)? == 0 {
                continue;
            }

            let command = command.trim();
            if command.is_empty() {
                continue;
            }

            let (parts, redirect) = match tokenize(command) {
                Ok(result) => result,
                Err(message) => {
                    eprintln!("{}", message);
                    continue;
                }
            };

            if parts.is_empty() {
                continue;
            }

            let mut redirect_file = match redirect {
                Some(spec) => {
                    if spec.fd != 1 {
                        eprintln!("redirect for fd {} is not supported", spec.fd);
                        continue;
                    }
                    match get_write_output(&spec.target) {
                        Ok(file) => Some(file),
                        Err(err) => {
                            eprintln!("failed to open {}: {}", spec.target, err);
                            continue;
                        }
                    }
                }
                None => None,
            };

            let command_name = parts[0].as_str();

            if let Some(builtin) = self.builtins.get(command_name) {
                let stdout = io::stdout();
                let mut writer = self.prepare_builtin_output(&stdout, redirect_file.as_ref())?;
                let flow = builtin(self, &parts, &mut writer)?;
                if let BuiltinFlow::Exit(code) = flow {
                    process::exit(code);
                }
                continue;
            }

            if find_executable(command_name).is_none() {
                let stdout = io::stdout();
                let mut writer = self.prepare_builtin_output(&stdout, redirect_file.as_ref())?;
                self.write_line(&mut writer, &format!("{}: command not found", command_name))?;
                continue;
            }

            if let Err(err) = self.run_external(&parts, redirect_file.take()) {
                eprintln!("{}", err);
            }
        }
    }

    fn prepare_builtin_output<'a>(
        &self,
        stdout: &'a io::Stdout,
        redirect: Option<&File>,
    ) -> io::Result<OutputSink<'a>> {
        if let Some(file) = redirect {
            Ok(OutputSink::File(file.try_clone()?))
        } else {
            Ok(OutputSink::Stdout(stdout.lock()))
        }
    }

    fn write_line(&self, writer: &mut dyn Write, content: &str) -> io::Result<()> {
        writer.write_all(content.as_bytes())?;
        writer.write_all(b"\n")
    }

    fn builtin_exit(
        &mut self,
        parts: &[String],
        writer: &mut dyn Write,
    ) -> io::Result<BuiltinFlow> {
        let status_code = if parts.len() > 1 {
            match parts[1].parse::<i32>() {
                Ok(code) => code,
                Err(_) => {
                    self.write_line(
                        writer,
                        &format!("exit: {}: numeric argument required", parts[1]),
                    )?;
                    return Ok(BuiltinFlow::Continue);
                }
            }
        } else {
            0
        };

        Ok(BuiltinFlow::Exit(status_code))
    }

    fn builtin_echo(
        &mut self,
        parts: &[String],
        writer: &mut dyn Write,
    ) -> io::Result<BuiltinFlow> {
        let message = parts[1..].join(" ");
        self.write_line(writer, &message)?;
        Ok(BuiltinFlow::Continue)
    }

    fn builtin_type(
        &mut self,
        parts: &[String],
        writer: &mut dyn Write,
    ) -> io::Result<BuiltinFlow> {
        if parts.len() != 2 {
            self.write_line(writer, "type only accepts 2 arguments")?;
            return Ok(BuiltinFlow::Continue);
        }

        let target = &parts[1];
        if self.builtins.contains_key(target.as_str()) {
            self.write_line(writer, &format!("{} is a shell builtin", target))?;
            return Ok(BuiltinFlow::Continue);
        }

        if let Some(path) = find_executable(target) {
            self.write_line(writer, &format!("{} is {}", target, path.display()))?;
        } else {
            self.write_line(writer, &format!("{}: not found", target))?;
        }

        Ok(BuiltinFlow::Continue)
    }

    fn builtin_pwd(
        &mut self,
        _parts: &[String],
        writer: &mut dyn Write,
    ) -> io::Result<BuiltinFlow> {
        match env::current_dir() {
            Ok(path) => {
                self.write_line(writer, &path.to_string_lossy())?;
            }
            Err(_) => {
                self.write_line(writer, "Can't find current directory")?;
            }
        }

        Ok(BuiltinFlow::Continue)
    }

    fn builtin_cd(
        &mut self,
        parts: &[String],
        writer: &mut dyn Write,
    ) -> io::Result<BuiltinFlow> {
        if parts.len() != 2 {
            self.write_line(writer, "cd only accepts 1 argument")?;
            return Ok(BuiltinFlow::Continue);
        }

        let mut new_dir = parts[1].clone();
        if parts[1].starts_with('~') {
            if let Ok(home_dir) = env::var("HOME") {
                let remainder = parts[1].trim_start_matches('~');
                if remainder.is_empty() {
                    new_dir = home_dir;
                } else {
                    new_dir = format!("{}/{}", home_dir, remainder.trim_start_matches('/'));
                }
            }
        }

        if env::set_current_dir(&new_dir).is_err() {
            self.write_line(writer, &format!("{}: No such file or directory", parts[1]))?;
        }

        Ok(BuiltinFlow::Continue)
    }

    fn run_external(
        &self,
        parts: &[String],
        redirect_file: Option<File>,
    ) -> io::Result<()> {
        let mut command = process::Command::new(&parts[0]);
        command.args(&parts[1..]);

        if let Some(file) = redirect_file {
            command.stdout(Stdio::from(file));
        }

        let mut child = command.spawn()?;
        let _status = child.wait()?;
        Ok(())
    }
}

fn main() {
    let mut shell = Shell::new();
    if let Err(err) = shell.run() {
        eprintln!("shell error: {}", err);
    }
}

fn find_executable(file_path_str: &str) -> Option<PathBuf> {
    let path_var = env::var("PATH").unwrap();
    let paths = path_var.split(':');
    for path in paths {
        let file_path_str = format!("{}/{}", path, file_path_str);
        let file_path = PathBuf::from(file_path_str);
        if let Ok(metadata) = file_path.metadata() {
            let permissions = metadata.permissions();
            let is_executable = permissions.mode() & 0o111 != 0;
            if metadata.is_file() && is_executable {
                return Some(file_path)
            }
        }
    }
    None
}
