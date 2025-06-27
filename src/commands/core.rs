use std::env;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::utils::{expand, expand_tilde};

// Alias storage
static ALIASES: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

// Environment variables storage
static ENV_VARS: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

pub fn set_alias(name: &str, cmd: &str) {
    let aliases = ALIASES.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = aliases.lock().unwrap();
    map.insert(name.to_string(), cmd.to_string());
}

pub fn lookup_alias(name: &str) -> Option<String> {
    ALIASES.get()
        .and_then(|aliases| {
            let map = aliases.lock().unwrap();
            map.get(name).cloned()
        })
}

pub fn expand_aliases(input: &str) -> String {
    let mut result = input.to_string();
    let mut depth = 0;
    const MAX_DEPTH: usize = 10;

    while depth < MAX_DEPTH {
        let parts: Vec<&str> = result.splitn(2, ' ').collect();
        if parts.is_empty() {
            break;
        }

        if let Some(alias_cmd) = lookup_alias(parts[0]) {
            let new_cmd = if parts.len() > 1 {
                // Replace parameters ($1, $2, ...)
                let mut expanded = alias_cmd.clone();
                for (i, arg) in parts[1].split_whitespace().enumerate() {
                    let param = format!("${}", i + 1);
                    expanded = expanded.replace(&param, arg);
                }
                format!("{} {}", expanded, parts[1])
            } else {
                alias_cmd.clone()
            };
            result = new_cmd;
            depth += 1;
        } else {
            break;
        }
    }

    if depth >= MAX_DEPTH {
        eprintln!("Alias expansion depth limit reached");
    }

    result
}

pub fn handle_alias_cmd(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        let aliases = ALIASES.get().unwrap().lock().unwrap();
        for (name, cmd) in aliases.iter() {
            println!("{}='{}'", name, cmd);
        }
        return Ok(());
    }

    // Join arguments into a single string
    let full = args.join(" ");
    let (name, value) = if let Some(eq_pos) = full.find('=') {
        let name = full[..eq_pos].trim();
        let value = full[eq_pos + 1..].trim().trim_matches('"');
        (name, value)
    } else if let Some(space_pos) = full.find(' ') {
        let name = full[..space_pos].trim();
        let value = full[space_pos + 1..].trim().trim_matches('"');
        (name, value)
    } else {
        return Err("Invalid alias syntax".into());
    };
    if name.is_empty() || value.is_empty() {
        return Err("Alias name and value cannot be empty".into());
    }

    set_alias(name, value);
    Ok(())
}

/// Change current working directory
pub fn cd(args: &[String]) -> Result<(), String> {
    let path = args.first().map(|s| s.as_str()).unwrap_or("~");
    let expanded = expand(path)?;
    
    // Use the new expand_tilde function
    let expanded_path = expand_tilde(&expanded)?;

    std::env::set_current_dir(&expanded_path)
        .map_err(|e| format!("cd: {}", e))?;
    
    Ok(())
}

pub fn execute_external_command(cmd: &str, args: &[String]) -> Result<(), String> {
    let mut command = std::process::Command::new(cmd);
    
    // 1. Get system environment variables
    let mut env_vars: HashMap<String, String> = env::vars().collect();
    
    // 2. Add custom variables from ENV_VARS
    if let Some(custom_vars) = ENV_VARS.get() {
        let map = custom_vars.lock().unwrap();
        for (key, value) in map.iter() {
            env_vars.insert(key.clone(), value.clone());
        }
    }
    
    // 3. Set all environment variables for the command
    command.envs(env_vars);
    
    // 4. Execute the command with arguments
    let status = command
        .args(args)
        .status()
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => format!("Command not found: {}", cmd),
            _ => format!("Failed to execute command: {}", e),
        })?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "Command failed with code {}",
            status.code().unwrap_or(-1)
        ))
    }
}

pub fn get_env_var(name: &str) -> Option<String> {
    // Try system environment first
    if let Ok(value) = env::var(name) {
        return Some(value);
    }
    
    // Try custom storage
    ENV_VARS.get().and_then(|env_vars| {
        let map = env_vars.lock().unwrap();
        map.get(name).cloned()
    })
}

/// Handle export command
pub fn handle_export_cmd(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        // Display all environment variables (system + custom)
        let mut all_vars: HashMap<String, String> = env::vars().collect();
        
        // Add custom variables
        if let Some(env_vars) = ENV_VARS.get() {
            let map = env_vars.lock().unwrap();
            for (key, value) in map.iter() {
                all_vars.insert(key.clone(), value.clone());
            }
        }
        
        // Find max key length for formatting
        let max_key_len = all_vars.keys().map(|k| k.len()).max().unwrap_or(0);
        let mut vars: Vec<_> = all_vars.into_iter().collect();
        vars.sort_by(|a, b| a.0.cmp(&b.0));
        
        // Print formatted output
        for (key, value) in vars {
            println!("{:<width$} {}", key, value, width = max_key_len);
        }
    } else {
        // Process arguments without holding lock
        let mut new_vars = Vec::new();
        for arg in args {
            if let Some(pos) = arg.find('=') {
                let name = arg[..pos].to_string();
                let value = arg[pos + 1..].to_string();
                new_vars.push((name, value));
            } else {
                return Err(format!("Invalid export syntax: {}", arg));
            }
        }
        
        // Update storage with new variables
        let env_vars = ENV_VARS.get_or_init(|| Mutex::new(HashMap::new()));
        let mut map = env_vars.lock().unwrap();
        for (name, value) in new_vars {
            map.insert(name, value);
        }
    }
    Ok(())
}

/// Display help information
pub fn help() -> Result<(), String> {
    println!("Shesh Shell - Built-in Commands");
    println!("--------------------------------");
    println!("  cd [dir]       - Change current directory");
    println!("  exit [code]    - Exit shell with optional exit code");
    println!("  help           - Show this help message");
    println!("\nSpecial Symbols:");
    println!("  > file         - Redirect output to file (overwrite)");
    println!("  >> file        - Redirect output to file (append)");
    println!("  < file         - Redirect input from file");
    println!("  cmd1 | cmd2    - Pipe output of cmd1 to input of cmd2");
    Ok(())
}
