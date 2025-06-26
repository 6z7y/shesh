use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use os_pipe::pipe;


/// Represents different types of special symbols in shell commands
#[derive(Debug, PartialEq)]
pub enum SymbolType {
    Pipe, // |
    RedirectOut, // >
    RedirectAppend, // >>
    RedirectIn, // <
    Background, // &
    AndAnd, // &&
    Semicolon // ;
}

impl SymbolType {
    /// Convert a token string to SymbolType
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "|" => Some(Self::Pipe),
            ">" => Some(Self::RedirectOut),
            ">>" => Some(Self::RedirectAppend),
            "<" => Some(Self::RedirectIn),
            "&" => Some(Self::Background),
            "&&" => Some(Self::AndAnd),
            ";" => Some(Self::Semicolon),
            _ => None
        }
    }
}

/// Handle all special symbols in a command
pub fn handle_symbols(tokens: &[String]) -> Result<(), String> {
    // Find the first special symbol in the tokens
    let symbol_pos = tokens.iter().enumerate().find_map(|(i, token)| {
        SymbolType::from_str(token).map(|sym| (i, sym))
    });

    match symbol_pos {
        Some((pos, symbol)) => match symbol {
            SymbolType::Pipe => handle_pipe(&tokens[..pos], &tokens[pos+1..]),
            SymbolType::RedirectOut | SymbolType::RedirectAppend | SymbolType::RedirectIn => {
                if tokens.len() <= pos + 1 {
                    return Err("Missing file argument".into());
                }
                handle_redirection(
                    match symbol {
                        SymbolType::RedirectOut => ">",
                        SymbolType::RedirectAppend => ">>",
                        SymbolType::RedirectIn => "<",
                        _ => unreachable!()
                    },
                    &tokens[..pos],
                    &tokens[pos+1]
                )
            }
            SymbolType::Background => {
                // Background symbol must be at the end
                if pos != tokens.len() - 1 {
                    return Err("Background symbol must be at the end".into());
                }
                handle_background(&tokens[..pos])
            }
            SymbolType::AndAnd => handle_and_and(&tokens[..pos], &tokens[pos+1..]),
            SymbolType::Semicolon => handle_semicolon(&tokens[..pos], &tokens[pos+1..]),
        },
        None => Err("No special symbol found".into()),
    }
}

/// Handle redirection symbols
pub fn handle_redirection(symbol:&str,cmd_tokens:&[String],file_name: &str)->Result<(),String>{
    if cmd_tokens.is_empty() {
        return Err("Missing command".into());
    }

    match symbol {
        ">" => {
            let mut file = File::create(file_name)
                .map_err(|e| format!("Error creating file: {}", e))?;
            
            let output = std::process::Command::new(&cmd_tokens[0])
                .args(&cmd_tokens[1..])
                .output()
                .map_err(|e| format!("Command failed: {}", e))?;
            
            if !output.status.success() {
                return Err(format!(
                    "Command failed with code {}",
                    output.status.code().unwrap_or(-1)
                ));
            }
            
            file.write_all(&output.stdout)
                .map_err(|e| format!("Error writing to file: {}", e))
        }
        ">>" => {
            let mut file = OpenOptions::new()
                .append(true)  // تم التصحيح هنا
                .create(true)
                .open(file_name)
                .map_err(|e| format!("Error opening file: {}", e))?;
            
            let output = std::process::Command::new(&cmd_tokens[0])
                .args(&cmd_tokens[1..])
                .output()
                .map_err(|e| format!("Command failed: {}", e))?;
            
            if !output.status.success() {
                return Err(format!(
                    "Command failed with code {}",
                    output.status.code().unwrap_or(-1)
                ));
            }
            
            file.write_all(&output.stdout)
                .map_err(|e| format!("Error writing to file: {}", e))
        }
        "<" => {
            let mut file = File::open(file_name)
                .map_err(|e| format!("Error opening file: {}", e))?;
            let mut contents = String::new();
            file.read_to_string(&mut contents)
                .map_err(|e| format!("Error reading file: {}", e))?;
            
            // Execute command with file contents as input
            let status = std::process::Command::new(&cmd_tokens[0])
                .args(&cmd_tokens[1..])
                .stdin(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| format!("Failed to start command: {}", e))?
                .wait()
                .map_err(|e| format!("Command failed: {}", e))?;
            
            if status.success() {
                Ok(())
            } else {
                Err(format!(
                    "Command failed with code {}",
                    status.code().unwrap_or(-1)
                ))
            }
        }
        _ => Err("Unknown redirection symbol".into()),
    }
}

/// Handle pipe symbol
pub fn handle_pipe(left_cmd: &[String], right_cmd: &[String]) -> Result<(), String> {
    if left_cmd.is_empty() || right_cmd.is_empty() {
        return Err("Both sides of pipe must contain commands".into());
    }

    let (reader, writer) = pipe().map_err(|e| e.to_string())?;

    // First command
    let mut child1 = std::process::Command::new(&left_cmd[0])
        .args(&left_cmd[1..])
        .stdout(writer)
        .spawn()
        .map_err(|e| format!("First command failed: {}", e))?;

    // Second command
    let mut child2 = std::process::Command::new(&right_cmd[0])
        .args(&right_cmd[1..])
        .stdin(reader)
        .spawn()
        .map_err(|e| format!("Second command failed: {}", e))?;

    // Wait for both commands
    let status1 = child1.wait().map_err(|e| e.to_string())?;
    let status2 = child2.wait().map_err(|e| e.to_string())?;

    if !status1.success() {
        return Err(format!(
            "Left command failed with code {}",
            status1.code().unwrap_or(-1)
        ));
    }

    if !status2.success() {
        return Err(format!(
            "Right command failed with code {}",
            status2.code().unwrap_or(-1)
        ));
    }

    Ok(())
}

/// Handle background execution
pub fn handle_background(cmd_tokens: &[String]) -> Result<(), String> {
    if cmd_tokens.is_empty() {
        return Err("Missing command".into());
    }

    // Clone tokens for thread
    let tokens = cmd_tokens.to_vec();
    
    // Spawn a new thread for background execution
    std::thread::spawn(move || {
        // Detach the process from terminal (Unix systems)
        #[cfg(unix)]
        unsafe {
            libc::setsid();
        }
        
        // Open /dev/null for stdout and stderr
        let dev_null = std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/null")
            .expect("Failed to open /dev/null");

        // Execute the command without background symbol
        let _ = std::process::Command::new(&tokens[0])
            .args(&tokens[1..])
            .stdout(dev_null.try_clone().unwrap()) // Redirect stdout to /dev/null
            .stderr(dev_null) // Redirect stderr to /dev/null
            .spawn()
            .and_then(|mut child| child.wait());
    });
    
    Ok(())
}

/// Handle && operator
pub fn handle_and_and(left_cmd: &[String], right_cmd: &[String]) -> Result<(), String> {
    // Execute left command
    let left_result = super::execute_command(left_cmd);
    
    // Only execute right command if left succeeded
    if left_result.is_ok() {
        super::execute_command(right_cmd)
    } else {
        left_result
    }
}

/// Handle ; operator
pub fn handle_semicolon(left_cmd: &[String], right_cmd: &[String]) -> Result<(), String> {
    // Always execute left command
    let _ = super::execute_command(left_cmd);
    
    // Always execute right command
    super::execute_command(right_cmd)
}
