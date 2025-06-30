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
    let tokens = shell_words::split(input)
        .map_err(|e| format!("Parse error: {}", e))?;
    
    let mut result = Vec::new();
    for token in tokens {
        let mut temp = Vec::new();
        let mut current = String::new();
        let mut chars = token.chars().peekable();
        
        while let Some(c) = chars.next() {
            // Handle combined symbols like 2>&1
            if c == '2' && chars.peek() == Some(&'>') {
                chars.next(); // Skip '>'
                if chars.peek() == Some(&'&') {
                    chars.next(); // Skip '&'
                    if chars.peek() == Some(&'1') {
                        chars.next(); // Skip '1'
                        if !current.is_empty() {
                            temp.push(current);
                            current = String::new();
                        }
                        temp.push("2>&1".to_string());
                        continue;
                    } else {
                        if !current.is_empty() {
                            temp.push(current);
                            current = String::new();
                        }
                        temp.push("2>".to_string());
                        continue;
                    }
                } else {
                    if !current.is_empty() {
                        temp.push(current);
                        current = String::new();
                    }
                    temp.push("2>".to_string());
                    continue;
                }
            }
            
            if ";|&<>".contains(c) {
                if !current.is_empty() {
                    temp.push(current);
                    current = String::new();
                }
                temp.push(c.to_string());
            } else {
                current.push(c);
            }
        }
        
        if !current.is_empty() {
            temp.push(current);
        }
        
        result.extend(temp);
    }
    
    Ok(result)
}
