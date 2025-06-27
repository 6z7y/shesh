//! Core shell execution logic
use crate::utils::expand;
use crate::commands;

/// Execute a shell command
pub fn execute(input: &str) -> Result<(), String> {
    // 1. Parse input
    let tokens = parse_input(input)?;
    
    // 2. Expand tokens
    let expanded_tokens = expand_tokens(&tokens)?;
    
    // 3. Execute the command with tokens
    commands::execute_command(&expanded_tokens)
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

pub fn parse_input(input: &str) -> Result<Vec<String>, String> {
    // Split input while preserving quoted strings
    shell_words::split(input)
        .map_err(|e| format!("Parse error: {}", e))
        .map(|tokens| {
            // Handle multi-character operators (&&)
            let mut result = Vec::new();
            for token in tokens {
                if token == "&" {
                    // Check if previous token is also '&'
                    if let Some(prev) = result.last_mut() {
                        if prev == "&" {
                            *prev = "&&".to_string();
                            continue;
                        }
                    }
                }
                result.push(token);
            }
            result
        }
    )
}
