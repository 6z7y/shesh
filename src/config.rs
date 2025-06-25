use std::{
    env,
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf
};

use crate::shell;

pub struct Config {
    pub prompt: Option<String>,
    pub startup: Vec<String>,
}

impl Config {
    fn default() -> Self {
        Self {
            prompt: Some("#shesh> ".to_string()),
            startup: Vec::new(),
        }
    }
}

// config

fn get_home_dir() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            eprintln!("Warning: HOME not set, using current directory");
            env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        })
}

fn get_config_path() -> PathBuf {
    get_home_dir().join(".config/shesh/shesh.24")
}

fn ensure_config_dirs(config_path: &std::path::Path) {
    if let Some(parent) = config_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
}


fn create_default_config(config_path: &PathBuf) {
    let default_content = "#prompt = \"shesh> \"\n#startup\necho \"shesh ready!\"";
    let _ = fs::write(config_path, default_content);
}

pub fn init() -> Config {
    let config_path = get_config_path();
    ensure_config_dirs(&config_path);
    
    if !config_path.exists() {
        create_default_config(&config_path);
    }
    
    load_config(&config_path)
}


fn load_config(path: &PathBuf) -> Config {
    let mut config = Config::default();
    
    let Ok(content) = fs::read_to_string(path) else {
        return config;
    };

    let mut in_startup = false;
    
    for line in content.lines() {
        let trimmed = line.trim();
        
        if trimmed.is_empty() {
            continue;
        }
        
        if let Some(comment) = trimmed.strip_prefix('#') {
            let commented_line = comment.trim();
            if commented_line.starts_with("prompt") {
                // Prompt is commented out - use default
                config.prompt = None;
            } else if commented_line.eq_ignore_ascii_case("startup") {
                in_startup = true;
            }
            continue;
        }
        
        if in_startup {
            config.startup.push(trimmed.to_string());
        } else if let Some((key, value)) = trimmed.split_once('=') {
            if key.trim() == "prompt" {
                // Prompt is not commented out - use custom prompt
                config.prompt = Some(value.trim().trim_matches('"').to_string());
            }
        }
    }
    config
}

pub fn run_startup(config: &Config) {
    for cmd_line in &config.startup {
        let parts = shell::parse_input(cmd_line).unwrap_or_default();
        if let Some((cmd, args)) = parts.split_first() {
            // Combine command and arguments into a single string for shell::execute
            let full_cmd = if args.is_empty() {
                cmd.clone()
            } else {
                format!("{} {}", cmd, args.join(" "))
            };
            
            if let Err(e) = shell::execute(&full_cmd) {
                eprintln!("Startup command failed: {}", e);
            }
        }
    }
}

//-----------------------
//history
pub fn history_file_path() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".local/share/shesh")
        .join("history")
}

// Append a command to the history file and return it if valid
pub fn append_to_history(command: &str) {
    let path = history_file_path();

   // Create parent directories if needed
    if let Some(parent) = path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            eprintln!("Failed to create directory: {}", e);
            return;
        }
    } 

    match OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        Ok(mut file) => {
            if let Err(e) = writeln!(file, "{}", command) {
                eprintln!("Failed to write to history: {}", e);
            }
        }
        Err(e) => eprintln!("Failed to open history file: {}", e),
    }
}
