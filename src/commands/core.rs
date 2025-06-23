use std::env;
use crate::utils::expand;

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
