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
    tokens
        .iter()
        .flat_map(|token| {
            // First expand braces
            let brace_expanded = expand_braces(token);
            
            // Then expand wildcards for each brace-expanded token
            brace_expanded.into_iter()
                .flat_map(|t| expand_wildcard(&t))
        })
        .collect()
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
pub fn handle_redirection(symbol: &str, cmd_tokens: &[String], file_name: &str) -> Result<(), String> {
    if cmd_tokens.is_empty() {
        return Err("Missing command".into());
    }

    // Expand wildcards in command tokens (not filename)
    let expanded_tokens: Vec<String> = cmd_tokens
        .iter()
        .flat_map(|token| expand_wildcard(token))
        .collect();

    match symbol {
        ">" => {
            let mut file = File::create(file_name)
                .map_err(|e| format!("Error creating file: {}", e))?;
            
            let output = std::process::Command::new(&expanded_tokens[0])
                .args(&expanded_tokens[1..])
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
                .append(true)
                .create(true)
                .open(file_name)
                .map_err(|e| format!("Error opening file: {}", e))?;
            
            let output = std::process::Command::new(&expanded_tokens[0])
                .args(&expanded_tokens[1..])
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
            
            let status = std::process::Command::new(&expanded_tokens[0])
                .args(&expanded_tokens[1..])
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

    let expanded_left = expand_tokens(left_cmd);
    let expanded_right = expand_tokens(right_cmd);

    let (reader, writer) = pipe().map_err(|e| e.to_string())?;

    let mut child1 = std::process::Command::new(&expanded_left[0])
        .args(&expanded_left[1..])
        .stdout(writer)
        .spawn()
        .map_err(|e| format!("First command failed: {}", e))?;

    let mut child2 = std::process::Command::new(&expanded_right[0])
        .args(&expanded_right[1..])
        .stdin(reader)
        .spawn()
        .map_err(|e| format!("Second command failed: {}", e))?;

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
