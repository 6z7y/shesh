//! Command execution engine

mod core;
mod symbols;
use std::process::{Command as StdCommand};

/// Execute a command (built-in, symbol, or external)
pub fn execute_command(cmd: &str, args: &[String]) -> Result<(), String> {
    // First handle special commands
    match cmd {
        // Then handle other built-ins
        "cd" => core::cd(args),
        "exit" => std::process::exit(0),
        "help" => core::help(),
        // Then handle symbols
        ">" | ">>" | "<" | "|" => handle_symbol(cmd, args),
        // Finally handle external commands
        _ => execute_external_command(cmd, args),
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
    // ... (نفس الكود السابق مع تعديلات طفيفة)
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
