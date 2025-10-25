use std::io::{self, Write};
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
            _ => {
                println!("{}: command not found", command);
            }
        }
    }
}
