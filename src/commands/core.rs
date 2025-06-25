use std::env;
use std::sync::Mutex;
use crate::utils::expand;

static ALIASES: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());

pub fn set_alias(name: &str, cmd: &str) {
    let mut m = ALIASES.lock().unwrap();
    if let Some((_, v)) = m.iter_mut().find(|(n, _)| n == name) {
        *v = cmd.to_string();
    } else {
        m.push((name.to_string(), cmd.to_string()));
    }
}
pub fn lookup_alias(name: &str) -> Option<String> {
    ALIASES
        .lock().unwrap()
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, v)| v.clone())
}

pub fn handle_alias_cmd(args: &[&str]) -> Result<(), String> {
    if args.is_empty() {
        for (n, c) in ALIASES.lock().unwrap().iter() {
            println!("alias {}=\"{}\"", n, c.replace('"', "\\\""));
        }
        return Ok(());
    }

    let full = args.join(" ");

    if let Some(eq_index) = full.find('=') {
        let name = &full[..eq_index];
        let mut cmd = &full[eq_index + 1..];
        cmd = cmd.trim();
        if cmd.starts_with('"') && cmd.ends_with('"') && cmd.len() > 1 {
            cmd = &cmd[1..cmd.len()-1];
        }
        set_alias(name, cmd);
        Ok(())
    } else if let Some(space_index) = full.find(' ') {
        let name = &full[..space_index];
        let mut cmd = &full[space_index + 1..];
        cmd = cmd.trim();
        if cmd.starts_with('"') && cmd.ends_with('"') && cmd.len() > 1 {
            cmd = &cmd[1..cmd.len()-1];
        }
        set_alias(name, cmd);
        Ok(())
    } else {
        Err(format!("Invalid alias usage: alias {}", full))
    }
}

pub fn expand_aliases(input: &str) -> String {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.is_empty() {
        return input.to_string();
    }

    if let Some(alias_cmd) = lookup_alias(parts[0]) {
        let rest = if parts.len() > 1 {
            format!(" {}", parts[1..].join(" "))
        } else {
            String::new()
        };
        format!("{}{}", alias_cmd, rest)
    } else {
        input.to_string()
    }
}

/// Change current working directory
pub fn cd(args: &[String]) -> Result<(), String> {
    let path = args.first().map(|s| s.as_str()).unwrap_or("~");
    let expanded = expand(path)?;
    
    env::set_current_dir(&expanded)
        .map_err(|e| format!("cd: {}", e))?;
    
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
