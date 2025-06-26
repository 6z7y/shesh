use std::sync::{Mutex, OnceLock};
use std::collections::HashMap;use crate::utils::expand;

static ALIASES: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

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
                // استبدال المعاملات ($1, $2, ...)
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

    // التعديل: استخدام join مباشرة على Vec<String>
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
    
    std::env::set_current_dir(&expanded)
        .map_err(|e| format!("cd: {}", e))?;
    
    Ok(())
}

pub fn execute_external_command(cmd: &str, args: &[String]) -> Result<(), String> {
    let status = std::process::Command::new(cmd)
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
