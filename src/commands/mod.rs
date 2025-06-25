//! Command execution engine

mod core;
mod symbols;
use std::process::{Command as StdCommand};

/// Execute a command (built-in, symbol, or external)
pub fn execute_command(cmd: &str, args: &[String]) -> Result<(), String> {
    // First, construct the full input string from cmd and args
    // Example: if cmd = "g" and args = ["status"], input = "g status"
    let input = if args.is_empty() {
        cmd.to_string()
    } else {
        format!("{} {}", cmd, args.join(" "))
    };

    // Expand aliases: e.g., if alias g="git", input becomes "git status"
    let expanded_input = core::expand_aliases(&input);

    // Split the expanded input into new command and arguments
    let mut parts = expanded_input.split_whitespace();
    let new_cmd = match parts.next() {
        Some(c) => c,
        None => return Ok(()), // No command to execute
    };
    let new_args: Vec<String> = parts.map(|s| s.to_string()).collect();
    let alias_args: Vec<&str> = new_args.iter().map(|s| s.as_str()).collect();


    // Match and execute built-in, symbol, or external command
    match new_cmd {
        "alias" => core::handle_alias_cmd(&alias_args),
        "cd" => core::cd(&new_args),
        "exit" => std::process::exit(0),
        "help" => core::help(),
        ">" | ">>" | "<" | "|" => handle_symbol(new_cmd, &new_args),
        _ => execute_external_command(new_cmd, &new_args),
    }
}

/// Handle symbol commands (redirection, pipes, etc.)
fn handle_symbol(symbol: &str, args: &[String]) -> Result<(), String> {
    match symbol {
        ">" | ">>" | "<" => handle_redirection(symbol, args),
        "|" => handle_pipe(args),
        _ => Err(format!("Unsupported symbol: {}", symbol)),
    }
}

/// Handle redirection symbols
fn handle_redirection(_symbol: &str, _args: &[String]) -> Result<(), String> {
    // Implementation for redirection handling
    Err("Redirection not implemented".to_string())
}

/// Handle pipe symbol
fn handle_pipe(_args: &[String]) -> Result<(), String> {
    // Implementation for pipe handling
    Err("Pipes not implemented".to_string())
}

/// Execute an external command
fn execute_external_command(cmd: &str, args: &[String]) -> Result<(), String> {
    let mut process = StdCommand::new(cmd);
    process.args(args);
    
    match process.status() {
        Ok(status) => {
            if status.success() {
                Ok(())
            } else {
                Err(format!("Command failed with code: {}", status.code().unwrap_or(-1)))
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Err(format!("Command not found: {}", cmd))
        }
        Err(e) => Err(format!("Failed to execute command: {}", e)),
    }
}
