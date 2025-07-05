use std::fs::{File, OpenOptions};
use std::process::{Stdio, Command};
use crate::utils::expand_tilde;
use crate::b_mod::execute_command;

/// Represents different types of special symbols in shell commands
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum SymbolType {
    Pipe,             // |
    RedirectOut,      // >
    RedirectAppend,   // >>
    RedirectIn,       // <
    RedirectErr,      // 2>
    RedirectErrAppend,// 2>>
    RedirectAll,      // &>
    RedirectAllAppend,// &>>
    Background,       // &
    AndAnd,           // &&
    Semicolon,        // ;
}

impl SymbolType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "|" => Some(Self::Pipe),
            ">" => Some(Self::RedirectOut),
            ">>" => Some(Self::RedirectAppend),
            "<" => Some(Self::RedirectIn),
            "1>" => Some(Self::RedirectOut),
            "1>>" => Some(Self::RedirectAppend),
            "2>" => Some(Self::RedirectErr),
            "2>>" => Some(Self::RedirectErrAppend),
            "&>" => Some(Self::RedirectAll),
            "&>>" => Some(Self::RedirectAllAppend),
            "&" => Some(Self::Background),
            "&&" => Some(Self::AndAnd),
            ";" => Some(Self::Semicolon),
            _ => None
        }
    }
}

/// Handle all special symbols in a command
pub fn handle_symbols(tokens: &[String]) -> Result<(), String> {
    let mut symbol_positions = Vec::new();
    let mut i = 0;
    
    while i < tokens.len() {
        if let Some(sym) = SymbolType::from_str(&tokens[i]) {
            symbol_positions.push((i, sym));
        }
        i += 1;
    }

    if symbol_positions.is_empty() {
        return Err("No special symbol found".into());
    }

    let (pos, symbol) = symbol_positions[0];
    match symbol {
        SymbolType::Pipe => handle_pipe(&tokens[..pos], &tokens[pos+1..]),
        SymbolType::RedirectOut | 
        SymbolType::RedirectAppend | 
        SymbolType::RedirectIn | 
        SymbolType::RedirectErr | 
        SymbolType::RedirectErrAppend | 
        SymbolType::RedirectAll | 
        SymbolType::RedirectAllAppend => {
            if tokens.len() <= pos + 1 {
                return Err("Missing file argument".into());
            }
            handle_redirection(
                symbol,
                &tokens[..pos],
                &tokens[pos+1]
            )?;
            
            if tokens.len() > pos + 2 {
                handle_symbols(&tokens[pos+2..])
            } else {
                Ok(())
            }
        }
        SymbolType::Background => {
            if pos != tokens.len() - 1 {
                return Err("Background symbol must be at the end".into());
            }
            handle_background(&tokens[..pos])
        }
        SymbolType::AndAnd => handle_and_and(&tokens[..pos], &tokens[pos+1..]),
        SymbolType::Semicolon => handle_semicolon(&tokens[..pos], &tokens[pos+1..]),
    }
}

/// Expand wildcards in a single token '*'
pub fn expand_wildcard(pattern: &str) -> Vec<String> {
    // If pattern contains no wildcard, return it as is
    if !pattern.contains('*') {
        return vec![pattern.to_string()];
    }

    // Split path into directory and file pattern
    let path = std::path::Path::new(pattern);
    let (dir, file_pattern) = if let Some(parent) = path.parent() {
        let dir = if parent.as_os_str().is_empty() {
            std::path::Path::new(".")
        } else {
            parent
        };
        let file_pattern = match path.file_name() {
            Some(name) => name.to_str().unwrap_or(""),
            None => return vec![pattern.to_string()],
        };
        (dir, file_pattern)
    } else {
        (std::path::Path::new("."), pattern)
    };

    // Read directory entries
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return vec![pattern.to_string()],
    };

    let mut matches = Vec::new();
    for entry in entries.flatten() {
        let os_file_name = entry.file_name();  // Store OsString first
        let file_name = match os_file_name.to_str() {
            Some(name) => name,
            None => continue,
        };

        // Skip hidden files unless explicitly requested
        if file_name.starts_with('.') && !file_pattern.starts_with('.') {
            continue;
        }

        // Check if filename matches the pattern
        if matches_pattern(file_pattern, file_name) {
            let full_path = dir.join(file_name);
            matches.push(full_path.to_string_lossy().into_owned());
        }
    }

    // Return original pattern if no matches found
    if matches.is_empty() {
        vec![pattern.to_string()]
    } else {
        matches
    }
}
/// Expand brace patterns like {a,b} into multiple strings
pub fn expand_braces(input: &str) -> Vec<String> {
    let mut result = vec![String::new()];
    let mut stack = vec![];
    let mut in_brace = false;

    for c in input.chars() {
        match c {
            '{' if !in_brace => {
                in_brace = true;
                stack.push(result);
                result = vec![String::new()];
            }
            '}' if in_brace => {
                in_brace = false;
                let parts: Vec<String> = result
                    .iter()
                    .flat_map(|s| s.split(','))
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                
                result = stack.pop().unwrap();
                result = result
                    .iter()
                    .flat_map(|prev| {
                        parts.iter().map(move |p| format!("{}{}", prev, p))
                    })
                    .collect();
            }
            _ => {
                for s in &mut result {
                    s.push(c);
                }
            }
        }
    }
    result
}

/// Expand both braces and wildcards in tokens
pub fn expand_tokens(tokens: &[String]) -> Vec<String> {
    let mut new_tokens = Vec::new();
    
    for token in tokens {
        // Separate attached symbols
        let mut temp = Vec::new();
        let mut current = String::new();
        
        for c in token.chars() {
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
        
        // Expand braces and wildcards for each token
        for t in temp {
            // Apply tilde expansion and convert to string
            let expanded_str = expand_tilde(&t)
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or(t);
            
            let brace_expanded = expand_braces(&expanded_str);
            for b in brace_expanded {
                new_tokens.extend(expand_wildcard(&b));
            }
        }
    }
    new_tokens
}

/// Check if filename matches a wildcard pattern
fn matches_pattern(pattern: &str, name: &str) -> bool {
    let pattern_chars: Vec<char> = pattern.chars().collect();
    let name_chars: Vec<char> = name.chars().collect();
    let mut p_idx = 0;
    let mut n_idx = 0;
    let mut star: Option<usize> = None;
    let mut star_match_idx = 0;

    while n_idx < name_chars.len() {
        if p_idx < pattern_chars.len() && pattern_chars[p_idx] == '*' {
            star = Some(p_idx);
            star_match_idx = n_idx;
            p_idx += 1;
        } else if p_idx < pattern_chars.len() && pattern_chars[p_idx] == name_chars[n_idx] {
            p_idx += 1;
            n_idx += 1;
        } else if let Some(star_idx) = star {
            p_idx = star_idx + 1;
            star_match_idx += 1;
            n_idx = star_match_idx;
        } else {
            return false;
        }
    }

    // Skip trailing stars
    while p_idx < pattern_chars.len() && pattern_chars[p_idx] == '*' {
        p_idx += 1;
    }

    p_idx == pattern_chars.len() && n_idx == name_chars.len()
}

/// Handle redirection symbols
pub fn handle_redirection(
    symbol: SymbolType,
    cmd_tokens: &[String],
    file_name: &str
) -> Result<(), String> {
    if cmd_tokens.is_empty() {
        return Err("Missing command".into());
    }

    let expanded_tokens = expand_tokens(cmd_tokens);
    let expanded_file = expand_tilde(file_name)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| file_name.to_string());

    let mut command = Command::new(&expanded_tokens[0]);
    command.args(&expanded_tokens[1..]);

    match symbol {
        SymbolType::RedirectOut => {
            let file = File::create(&expanded_file)
                .map_err(|e| e.to_string())?;
            command.stdout(file);
        }
        SymbolType::RedirectAppend => {
            let file = OpenOptions::new()
                .append(true)
                .create(true)
                .open(&expanded_file)
                .map_err(|e| e.to_string())?;
            command.stdout(file);
        }
        SymbolType::RedirectErr => {
            let file = File::create(&expanded_file)
                .map_err(|e| e.to_string())?;
            command.stderr(file);
        }
        SymbolType::RedirectErrAppend => {
            let file = OpenOptions::new()
                .append(true)
                .create(true)
                .open(&expanded_file)
                .map_err(|e| e.to_string())?;
            command.stderr(file);
        }
        SymbolType::RedirectAll => {
            let file = File::create(&expanded_file)
                .map_err(|e| e.to_string())?;
            command.stdout(file.try_clone().map_err(|e| e.to_string())?);
            command.stderr(file);
        }
        SymbolType::RedirectAllAppend => {
            let file = OpenOptions::new()
                .append(true)
                .create(true)
                .open(&expanded_file)
                .map_err(|e| e.to_string())?;
            command.stdout(file.try_clone().map_err(|e| e.to_string())?);
            command.stderr(file);
        }
        _ => return Err("Unsupported redirection".into()),
    }

    let status = command.status()
        .map_err(|e| e.to_string())?;
        
    if status.success() {
        Ok(())
    } else {
        Err(format!("Command failed with code {}", status.code().unwrap_or(-1)))
    }
}

/// Handle pipe symbol
pub fn handle_pipe(left_cmd: &[String], right_cmd: &[String]) -> Result<(), String> {
    // Collect all commands in the pipeline
    let mut commands = vec![left_cmd.to_vec()];
    let mut current = Vec::new();
    
    // Split right_cmd into individual commands
    for token in right_cmd {
        if token == "|" {
            commands.push(current);
            current = Vec::new();
        } else {
            current.push(token.clone());
        }
    }
    commands.push(current);

    // Validate command count
    if commands.len() < 2 {
        return Err("Both sides of pipe must contain commands".into());
    }

    let mut children = vec![];
    let mut prev_stdout = None;

    for (i, cmd_tokens) in commands.iter().enumerate() {
        // Expand tokens for this command
        let expanded_tokens = expand_tokens(cmd_tokens);
        if expanded_tokens.is_empty() {
            return Err("Empty command in pipeline".into());
        }

        let mut command = Command::new(&expanded_tokens[0]);
        command.args(&expanded_tokens[1..]);

        // Set stdin from previous command if available
        if i > 0 {
            command.stdin(prev_stdout.take().unwrap());
        }

        // Set stdout to pipe if not last command
        if i < commands.len() - 1 {
            command.stdout(Stdio::piped());
        }

        // Spawn the command
        let mut child = command.spawn()
            .map_err(|e| format!("Failed to spawn command '{}': {}", expanded_tokens[0], e))?;

        // Capture stdout for next command
        if i < commands.len() - 1 {
            prev_stdout = child.stdout.take();
        }

        children.push(child);
    }

    // Wait for all child processes to finish
    let mut last_status = None;
    for child in children.iter_mut().rev() {
        let status = child.wait().map_err(|e| e.to_string())?;
        if last_status.is_none() {
            last_status = Some(status);
        }
    }

    // Return based on last command's status
    match last_status {
        Some(status) if status.success() => Ok(()),
        Some(status) => Err(format!(
            "Pipeline failed with code {}",
            status.code().unwrap_or(-1)
        )), // Fixed: added closing parenthesis here
        None => Ok(()),
    }
}

/// Handle background execution
pub fn handle_background(cmd_tokens: &[String]) -> Result<(), String> {
    if cmd_tokens.is_empty() {
        return Err("Missing command".into());
    }

    let expanded_tokens = expand_tokens(cmd_tokens);
    let tokens = expanded_tokens.to_vec();
    
    std::thread::spawn(move || {
        #[cfg(unix)]
        unsafe {
            libc::setsid();
        }
        
        let dev_null = std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/null")
            .expect("Failed to open /dev/null");

        let _ = std::process::Command::new(&tokens[0])
            .args(&tokens[1..])
            .stdout(dev_null.try_clone().unwrap())
            .stderr(dev_null)
            .spawn()
            .and_then(|mut child| child.wait());
    });
    
    Ok(())
}

/// Handle && operator
pub fn handle_and_and(left_cmd: &[String], right_cmd: &[String]) -> Result<(), String> {
    // Execute left command
    let left_result = execute_command(left_cmd);
    
    // Only execute right command if left succeeded
    if left_result.is_ok() {
        execute_command(right_cmd)
    } else {
        left_result
    }
}

/// Handle ; operator
pub fn handle_semicolon(left_cmd: &[String], right_cmd: &[String]) -> Result<(), String> {
    // Always execute left command
    let _ = execute_command(left_cmd);
    
    // Always execute right command
    execute_command(right_cmd)
}
