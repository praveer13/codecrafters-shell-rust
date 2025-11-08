use std::fs::File;
use std::io::{self, Write};
use std::process::{self, Stdio};

use crate::builtins::{BuiltinFlow, Builtins};
use crate::io_helpers::{get_write_output, OutputSink};
use crate::parser::tokenize;
use crate::utils::{find_executable, write_line};

pub struct Shell {
    builtins: Builtins,
}

impl Shell {
    pub fn new() -> Self {
        Shell {
            builtins: Builtins::new(),
        }
    }

    pub fn run(&mut self) -> io::Result<()> {
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

            let (mut stdout_redirect_file, mut stderr_redirect_file) = match redirect {
                Some(spec) => match spec.fd {
                    1 => match get_write_output(&spec.target, spec.redirect_type.clone()) {
                        Ok(file) => (Some(file), None),
                        Err(err) => {
                            eprintln!("failed to open {}: {}", spec.target, err);
                            continue;
                        }
                    },
                    2 => match get_write_output(&spec.target, spec.redirect_type.clone()) {
                        Ok(file) => (None, Some(file)),
                        Err(err) => {
                            eprintln!("failed to open {}: {}", spec.target, err);
                            continue;
                        }
                    },
                    _ => {
                        eprintln!("redirect for fd {} is not supported", spec.fd);
                        continue;
                    }
                },
                None => (None, None),
            };

            let command_name = parts[0].as_str();

            if let Some(builtin) = self.builtins.get(command_name) {
                let stdout = io::stdout();
                let stderr = io::stderr();
                let mut stdout_writer = self
                    .prepare_builtin_output(stdout_redirect_file.as_ref(), || {
                        OutputSink::Stdout(stdout.lock())
                    })?;
                let mut stderr_writer = self
                    .prepare_builtin_output(stderr_redirect_file.as_ref(), || {
                        OutputSink::Stderr(stderr.lock())
                    })?;
                let flow = builtin(
                    &self.builtins,
                    &parts,
                    &mut stdout_writer,
                    &mut stderr_writer,
                )?;
                if let BuiltinFlow::Exit(code) = flow {
                    process::exit(code);
                }
                continue;
            }

            if find_executable(command_name).is_none() {
                let stderr = io::stderr();
                let mut writer = self
                    .prepare_builtin_output(stderr_redirect_file.as_ref(), || {
                        OutputSink::Stderr(stderr.lock())
                    })?;
                write_line(&mut writer, &format!("{}: command not found", command_name))?;
                continue;
            }

            if let Err(err) = self.run_external(
                &parts,
                stdout_redirect_file.take(),
                stderr_redirect_file.take(),
            ) {
                eprintln!("{}", err);
            }
        }
    }

    fn prepare_builtin_output<'a, F>(
        &self,
        redirect: Option<&File>,
        fallback: F,
    ) -> io::Result<OutputSink<'a>>
    where
        F: FnOnce() -> OutputSink<'a>,
    {
        if let Some(file) = redirect {
            Ok(OutputSink::File(file.try_clone()?))
        } else {
            Ok(fallback())
        }
    }

    fn run_external(
        &self,
        parts: &[String],
        stdout_redirect_file: Option<File>,
        stderr_redirect_file: Option<File>,
    ) -> io::Result<()> {
        let mut command = process::Command::new(&parts[0]);
        command.args(&parts[1..]);

        if let Some(file) = stdout_redirect_file {
            command.stdout(Stdio::from(file));
        }
        if let Some(file) = stderr_redirect_file {
            command.stderr(Stdio::from(file));
        }

        let mut child = command.spawn()?;
        let _status = child.wait()?;
        Ok(())
    }
}
