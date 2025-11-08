use std::collections::HashMap;
use std::env;
use std::io::{self, Write};

use crate::utils::{find_executable, write_line};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinFlow {
    Continue,
    Exit(i32),
}

pub type BuiltinFn =
    fn(&Builtins, &[String], &mut dyn Write, &mut dyn Write) -> io::Result<BuiltinFlow>;

pub struct Builtins {
    registry: HashMap<&'static str, BuiltinFn>,
}

impl Builtins {
    pub fn new() -> Self {
        let mut registry: HashMap<&'static str, BuiltinFn> = HashMap::new();
        registry.insert("exit", Builtins::builtin_exit);
        registry.insert("echo", Builtins::builtin_echo);
        registry.insert("type", Builtins::builtin_type);
        registry.insert("pwd", Builtins::builtin_pwd);
        registry.insert("cd", Builtins::builtin_cd);
        Builtins { registry }
    }

    pub fn get(&self, name: &str) -> Option<&BuiltinFn> {
        self.registry.get(name)
    }

    pub fn is_builtin(&self, name: &str) -> bool {
        self.registry.contains_key(name)
    }

    fn builtin_exit(
        &self,
        parts: &[String],
        _stdout_writer: &mut dyn Write,
        stderr_writer: &mut dyn Write,
    ) -> io::Result<BuiltinFlow> {
        let status_code = if parts.len() > 1 {
            match parts[1].parse::<i32>() {
                Ok(code) => code,
                Err(_) => {
                    write_line(
                        stderr_writer,
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
        &self,
        parts: &[String],
        stdout_writer: &mut dyn Write,
        _stderr_writer: &mut dyn Write,
    ) -> io::Result<BuiltinFlow> {
        let message = parts[1..].join(" ");
        write_line(stdout_writer, &message)?;
        Ok(BuiltinFlow::Continue)
    }

    fn builtin_type(
        &self,
        parts: &[String],
        stdout_writer: &mut dyn Write,
        stderr_writer: &mut dyn Write,
    ) -> io::Result<BuiltinFlow> {
        if parts.len() != 2 {
            write_line(stderr_writer, "type only accepts 2 arguments")?;
            return Ok(BuiltinFlow::Continue);
        }

        let target = &parts[1];
        if self.is_builtin(target.as_str()) {
            write_line(stdout_writer, &format!("{target} is a shell builtin"))?;
            return Ok(BuiltinFlow::Continue);
        }

        if let Some(path) = find_executable(target) {
            write_line(stdout_writer, &format!("{target} is {}", path.display()))?;
        } else {
            write_line(stderr_writer, &format!("{target}: not found"))?;
        }

        Ok(BuiltinFlow::Continue)
    }

    fn builtin_pwd(
        &self,
        _parts: &[String],
        stdout_writer: &mut dyn Write,
        stderr_writer: &mut dyn Write,
    ) -> io::Result<BuiltinFlow> {
        match env::current_dir() {
            Ok(path) => {
                write_line(stdout_writer, &path.to_string_lossy())?;
            }
            Err(_) => {
                write_line(stderr_writer, "Can't find current directory")?;
            }
        }

        Ok(BuiltinFlow::Continue)
    }

    fn builtin_cd(
        &self,
        parts: &[String],
        _stdout_writer: &mut dyn Write,
        stderr_writer: &mut dyn Write,
    ) -> io::Result<BuiltinFlow> {
        if parts.len() != 2 {
            write_line(stderr_writer, "cd only accepts 1 argument")?;
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
            write_line(
                stderr_writer,
                &format!("{}: No such file or directory", parts[1]),
            )?;
        }

        Ok(BuiltinFlow::Continue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parts(words: &[&str]) -> Vec<String> {
        words.iter().map(|word| word.to_string()).collect()
    }

    #[test]
    fn echo_writes_joined_message() {
        let builtins = Builtins::new();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let flow = Builtins::builtin_echo(
            &builtins,
            &parts(&["echo", "hello", "world"]),
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(BuiltinFlow::Continue, flow);
        assert_eq!("hello world\n", String::from_utf8(stdout).unwrap());
        assert!(stderr.is_empty());
    }

    #[test]
    fn exit_with_invalid_argument_reports_error() {
        let builtins = Builtins::new();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let flow = Builtins::builtin_exit(
            &builtins,
            &parts(&["exit", "oops"]),
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(BuiltinFlow::Continue, flow);
        assert_eq!(
            "exit: oops: numeric argument required\n",
            String::from_utf8(stderr).unwrap()
        );
        assert!(stdout.is_empty());
    }

    #[test]
    fn type_reports_builtin() {
        let builtins = Builtins::new();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let flow = Builtins::builtin_type(
            &builtins,
            &parts(&["type", "echo"]),
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(BuiltinFlow::Continue, flow);
        assert_eq!(
            "echo is a shell builtin\n",
            String::from_utf8(stdout).unwrap()
        );
        assert!(stderr.is_empty());
    }
}
