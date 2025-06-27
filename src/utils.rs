use crate::commands::core::get_env_var;
use std::path::PathBuf;

pub fn expand_tilde(path: &str) -> Result<PathBuf, String> {
    if path.starts_with('~') {
        if let Some(rest) = path.strip_prefix('~') {
            if let Some(home) = get_env_var("HOME") {
                Ok(PathBuf::from(home).join(rest.trim_start_matches('/')))
            } else {
                Err("HOME environment variable not set".into())
            }
        } else {
            Ok(PathBuf::from(path))
        }
    } else {
        Ok(PathBuf::from(path))
    }
}

pub fn expand(input: &str) -> Result<String, String> {
    let mut output = String::new();
    let mut chars = input.chars().peekable();
    
    while let Some(c) = chars.next() {
        if c == '$' {
            // Handle variable expansion
            let mut var_name = String::new();
            
            // Check for ${...} syntax
            if let Some('{') = chars.peek() {
                chars.next(); // Skip '{'
                
                while let Some(c) = chars.next() {
                    if c == '}' {
                        break;
                    }
                    var_name.push(c);
                }
            } else {
                // Simple $NAME syntax
                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric() || c == '_' {
                        var_name.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
            }
            
            if var_name.is_empty() {
                output.push('$');
            } else if let Some(value) = get_env_var(&var_name) {
                output.push_str(&value);
            } else {
                return Err(format!("environment variable '{}' not found", var_name));
            }
        } else {
            output.push(c);
        }
    }
    
    Ok(output)
}
