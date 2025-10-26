use std::env;
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{PathBuf};
use std::process;

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut command = String::new();
        if io::stdin().read_line(&mut command).is_err() {
            eprintln!("failed to read input");
            continue;
        }

        let command = command.trim();
        let parts: Vec<&str> = command.split_whitespace().collect();

        if parts.is_empty() {
            continue;
        }

        let valid_commands = vec!["exit", "echo", "type"];
        match parts[0] {
            "exit" => {
                let status_code = if parts.len() > 1 {
                    match parts[1].parse::<i32>() {
                        Ok(code) => code,
                        Err(_) => {
                            println!("exit: {}: numeric argument required", parts[1]);
                            continue;
                        }
                    }
                } else {
                    0
                };

                process::exit(status_code);
            }
            "echo" => {
                let output = parts[1..].join(" ");
                println!("{}", output);
            }
            "type" => {
                if parts.len() > 2 {
                    eprintln!("type only accepts 2 arguments");
                    continue;
                }
                if valid_commands.iter().any(|s| s == &parts[1]) {
                    println!("{} is a shell builtin", parts[1]);
                } else {
                    let file_path_buf = find_executable(parts[1]);
                    if let Some(file_path) = file_path_buf {
                        println!("{} is {}", parts[1], file_path.display());
                    } else {
                        println!("{}: not found", parts[1]);
                    }
                }
            }
            _ => {
                let file_path_buf = find_executable(command);
                if let Some(file_path) = file_path_buf {
                    let mut child = process::Command::new(file_path)
                        .args(&parts[1..])
                        .spawn()
                        .expect("failed to execute command");

                    child.wait().expect("failed to wait on child");
                }
                println!("{}: command not found", command);
            }
        }
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