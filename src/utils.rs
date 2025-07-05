use crate::b_core::get_env_var; // تحديث المسار
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

lazy_static::lazy_static! {
    pub static ref VIM_ENABLED: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
}

pub fn toggle_vim_mode() -> bool {
    let mut enabled = VIM_ENABLED.lock().unwrap();
    *enabled = !*enabled;
    *enabled
}

pub fn vim_enabled() -> bool {
    *VIM_ENABLED.lock().unwrap()
}

pub fn expand_tilde(path: &str) -> Result<PathBuf, String> {
    if path.starts_with('~') {
        if let Some(home) = std::env::home_dir() {
            let home_str = home.to_string_lossy().into_owned();
            let expanded = path.replacen('~', &home_str, 1);
            Ok(PathBuf::from(expanded))
        } else if let Some(home) = get_env_var("HOME") {
            let home_str = home; // حل مشكلة الحجم
            let expanded = path.replacen('~', &home_str, 1);
            Ok(PathBuf::from(expanded))
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
    let mut in_double_quotes = false;
    let mut in_single_quotes = false;
    let mut escape_next = false;
    
    while let Some(c) = chars.next() {
        if escape_next {
            output.push(c);
            escape_next = false;
            continue;
        }
        
        match c {
            '\\' => {
                escape_next = true;
            }
            '"' if !in_single_quotes => {
                in_double_quotes = !in_double_quotes;
            }
            '\'' if !in_double_quotes => {
                in_single_quotes = !in_single_quotes;
            }
            '$' if !in_single_quotes => {
                let mut var_name = String::new();
                
                if let Some('{') = chars.peek() {
                    chars.next();
                    
                    while let Some(c) = chars.next() {
                        if c == '}' {
                            break;
                        }
                        var_name.push(c);
                    }
                } else {
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
                    let value_str = value; // حل مشكلة الحجم
                    output.push_str(&value_str);
                } else {
                    return Err(format!("environment variable '{}' not found", var_name));
                }
            }
            _ => {
                output.push(c);
            }
        }
    }
    
    if in_double_quotes || in_single_quotes {
        return Err("Unclosed quotes".into());
    }
    
    Ok(output)
}
