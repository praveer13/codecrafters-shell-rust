#[allow(unused_imports)]
use std::io::{self, Write};
use std::process;

fn main() {
    // Uncomment this block to pass the first stage
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        // Wait for user input
        let mut command = String::new();
        io::stdin().read_line(&mut command).unwrap();
        
        let command = command.trim();
        let parts: Vec<&str> = command.split_whitespace().collect();
        
        if parts.is_empty() {
            continue;
        }
        
        // Handle exit command
        if parts[0] == "exit" {
            let status_code = if parts.len() > 1 {
                // Parse the status code from the second argument
                match parts[1].parse::<i32>() {
                    Ok(code) => code,
                    Err(_) => {
                        println!("exit: {}: numeric argument required", parts[1]);
                        continue;
                    }
                }
            } else {
                // Default exit status is 0
                0
            };
            
            process::exit(status_code);
        }
        
        println!("{}: command not found", command);
    }
}
