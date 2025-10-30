use std::env;
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{PathBuf};
use std::process;

fn tokenize(input: &str) -> Vec<String> {
    let mut current_token: String = String::new();
    let mut tokens: Vec<String> = Vec::new();
    let mut input_chars = input.chars();
    let mut is_in_quotes = false;
    while let Some(ch) = input_chars.next() {
        match ch {
            '\'' => {
                is_in_quotes = !is_in_quotes;
            }
            ' ' | '\t' | '\n' => {
                if is_in_quotes {
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
        tokens.push(current_token.clone());
    }
    return tokens;
}

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
        let parts = tokenize(command);

        if parts.is_empty() {
            continue;
        }

        let valid_commands = vec!["exit", "echo", "type", "pwd", "cd"];
        match parts[0].as_str() {
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
                    let file_path_buf = find_executable(&parts[1]);
                    if let Some(file_path) = file_path_buf {
                        println!("{} is {}", parts[1], file_path.display());
                    } else {
                        println!("{}: not found", parts[1]);
                    }
                }
            }
            "pwd" => {
                if let Ok(current_dir) = env::current_dir() {
                    println!("{}", current_dir.to_string_lossy());
                } else {
                    eprintln!("Can't find current directory");
                }
            }
            "cd" => {
                if parts.len() == 2 {
                    let mut new_dir = parts[1].to_string();
                    if parts[1].starts_with("~") {
                        let home_dir = env::var("HOME").unwrap();
                        let path_children: Vec<&str> = parts[1].split("/").collect();
                        if path_children.len() > 1 {
                            let path_after_home = path_children[1..].join("/");
                            new_dir = format!("{}/{}",&home_dir, &path_after_home);
                        } else {
                            new_dir = home_dir;
                        }
                        
                    }
                    if env::set_current_dir(new_dir).is_err() {
                        eprintln!("{}: No such file or directory", parts[1]);
                    }
                } else {
                    eprintln!("cd only accepts 1 argument");
                }
                
            }
            _ => {
                let file_path_buf = find_executable(&parts[0]);
                if let Some(_file_path) = file_path_buf {
                    let mut child = process::Command::new(&parts[0])
                        .args(&parts[1..])
                        .spawn()
                        .expect("failed to execute command");

                    child.wait().expect("failed to wait on child");
                } else {
                    println!("{}: command not found", parts[0]);
                }
                
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
