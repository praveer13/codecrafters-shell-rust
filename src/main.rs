use std::io::{self, Write};
use std::path::Path;
use std::env;
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
                    let path_var = env::var("PATH").unwrap();
                    let paths = path_var.split(':');
                    let mut command_exists = false;
                    for path in paths {
                        let file_path_str = format!("{}/{}", path, parts[1]);
                        let file_path = Path::new(&file_path_str);
                        if file_path.exists() {
                            println!("{} is {}", parts[1], file_path_str);
                            command_exists = true;
                            break;
                        }
                    }
                    if !command_exists {
                        println!("{}: not found", parts[1]);
                    }
                }
            }
            _ => {
                println!("{}: command not found", command);
            }
        }
    }
}
