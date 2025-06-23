//! Core shell execution logic

use crate::commands::execute_command;
use crate::utils::expand;

/// Execute a shell command
pub fn execute(input: &str) -> Result<(), String> {
    // 1. Parse input
    let tokens = parse_input(input)?;
    
    // 2. Expand tokens
    let expanded = expand_tokens(&tokens)?;
    
    // 3. Split into command and arguments
    let (command, arguments) = split_command(&expanded)?;
    
    // 4. Execute the command
    execute_command(command, arguments)
}

/// Parse input string into tokens
pub fn parse_input(input: &str) -> Result<Vec<String>, String> {
    shell_words::split(input)
        .map_err(|e| format!("Parse error: {}", e))
}

/// Expand variables and special symbols in tokens
fn expand_tokens(tokens: &[String]) -> Result<Vec<String>, String> {
    tokens.iter()
        .map(|token| {
            // Expand tokens while preserving quoted strings
            if token.starts_with('"') && token.ends_with('"') {
                Ok(token.clone())
            } else {
                expand(token)
            }
        })
        .collect()
}

/// Split tokens into command and arguments
fn split_command(tokens: &[String]) -> Result<(&str, &[String]), String> {
    tokens.split_first()
        .map(|(cmd, args)| (cmd.as_str(), args))
        .ok_or_else(|| "Empty command after expansion".to_string())
}
