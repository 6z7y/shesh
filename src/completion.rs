use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::{BufReader, BufWriter, BufRead, Write};
use std::path::PathBuf;
use std::process::Command;
use reedline::{Completer, Suggestion, Span};
use dirs;

/// Main completer struct that handles command completions
pub struct MyCompleter {
    /// All available commands (builtins + PATH commands)
    commands: HashSet<String>,
    /// Directory to store completion cache files
    cache_dir: PathBuf,
    /// In-memory cache for subcommands
    subcommand_cache: HashMap<String, Vec<String>>,
}

impl MyCompleter {
    /// Initialize a new completer with default settings
    pub fn new() -> Self {
        let cache_dir = dirs::cache_dir()
            .expect("Failed to get cache directory")
            .join("shesh/completions");
        
        // Create cache directory if it doesn't exist
        fs::create_dir_all(&cache_dir).expect("Failed to create cache directory");

        Self {
            commands: Self::load_commands(),
            cache_dir,
            subcommand_cache: HashMap::new(),
        }
    }

    /// Load all available commands from PATH and builtins
    pub fn load_commands() -> HashSet<String> {
        let mut commands = HashSet::new();

        // Load commands from system PATH
        if let Some(path_var) = std::env::var_os("PATH") {
            for path in std::env::split_paths(&path_var) {
                if let Ok(entries) = std::fs::read_dir(path) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        if let Some(file_name) = entry.file_name().to_str().map(|s| s.to_string()) {
                            commands.insert(file_name);
                        }
                    }
                }
            }
        }

        // Add built-in commands
        commands.insert("cd".to_string());
        commands.insert("exit".to_string());
        commands.insert("help".to_string());

        commands
    }

    /// Get cache file path for a specific command
    fn get_cache_path(&self, cmd: &str) -> PathBuf {
        self.cache_dir.join(format!("{}.24", sanitize_filename(cmd)))
    }

    /// Load cache for a command from disk
    fn load_subcommands(&mut self, cmd: &str) -> Vec<String> {
        // Check in-memory cache first
        if let Some(subcommands) = self.subcommand_cache.get(cmd) {
            return subcommands.clone();
        }

        // Load from disk if not in memory
        let cache_file = self.get_cache_path(cmd);
        if let Ok(file) = OpenOptions::new().read(true).open(&cache_file) {
            let subcommands: Vec<String> = BufReader::new(file)
                .lines()
                .filter_map(|line| line.ok())
                .collect();
            
            if !subcommands.is_empty() {
                self.subcommand_cache.insert(cmd.to_string(), subcommands.clone());
                return subcommands;
            }
        }
        
        Vec::new()
    }

    /// Save command cache to disk
    fn save_subcommands(&self, cmd: &str, subcommands: &[String]) -> Result<(), std::io::Error> {
        let cache_file = self.get_cache_path(cmd);
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(cache_file)?;
        
        let mut writer = BufWriter::new(file);
        for subcmd in subcommands {
            writeln!(writer, "{}", subcmd)?;
        }
        Ok(())
    }

    /// Extract subcommands by running `cmd --help` and parsing output
    fn extract_subcommands(&self, cmd: &str) -> Vec<String> {
        let output = match Command::new(cmd).arg("--help").output() {
            Ok(output) => output,
            Err(_) => return Vec::new(),
        };

        let help_text = String::from_utf8_lossy(&output.stdout);
        let mut subcommands: Vec<String> = Vec::new();
        
        for line in help_text.lines() {
            if line.starts_with("  ") && !line.starts_with("  -") {
                if let Some(subcmd) = line.split_whitespace().next() {
                    // Filter out invalid subcommands
                    if !subcmd.starts_with('-') 
                        && subcmd != cmd 
                        && subcmd.len() > 1 
                        && !subcmd.contains('[') 
                        && !subcmd.contains('(') 
                    {
                        subcommands.push(subcmd.to_string());
                    }
                }
            }
        }

        // Remove duplicates and sort
        subcommands.sort();
        subcommands.dedup();
        subcommands
    }

    /// Get subcommands for a command (uses cache when possible)
    fn get_subcommands(&mut self, cmd: &str) -> Vec<String> {
        // Try to load from cache first
        let mut subcommands = self.load_subcommands(cmd);
        
        // Extract fresh subcommands if cache is empty
        if subcommands.is_empty() {
            subcommands = self.extract_subcommands(cmd);
            if !subcommands.is_empty() {
                self.save_subcommands(cmd, &subcommands).ok();
                self.subcommand_cache.insert(cmd.to_string(), subcommands.clone());
            }
        }
        
        subcommands
    }

    /// Handle file/directory completions
    fn complete_files(&self, current_word: &str, span: Span) -> Vec<Suggestion> {
        // Find the last slash to determine base directory and partial filename
        let last_slash = current_word.rfind('/').map(|i| i + 1).unwrap_or(0);
        let (base_str, partial) = current_word.split_at(last_slash);
        
        // Expand tilde in base directory
        let expanded_base = if base_str.starts_with('~') {
            expand_tilde(base_str)
        } else {
            PathBuf::from(base_str)
        };
        
        // Skip if base is not a directory
        if !expanded_base.is_dir() {
            return Vec::new();
        }
        
        // Read directory and generate suggestions
        match std::fs::read_dir(&expanded_base) {
            Ok(entries) => {
                let mut suggestions = Vec::new();
                let partial_span = Span::new(span.start + last_slash, span.end);
                
                for entry in entries.filter_map(|e| e.ok()) {
                    if let Some(file_name) = entry.file_name().to_str().map(|s| s.to_string()) {
                        // Skip hidden files unless explicitly requested
                        if !partial.starts_with('.') && file_name.starts_with('.') {
                            continue;
                        }
                        
                        if file_name.starts_with(partial) {
                            let is_dir = entry.path().is_dir();
                            let mut value = file_name.clone();
                            
                            if is_dir {
                                value.push('/');
                            }
                            
                            suggestions.push(Suggestion {
                                value,
                                description: None,
                                extra: None,
                                span: partial_span,
                                append_whitespace: false,
                                style: None,
                            });
                        }
                    }
                }
                suggestions
            }
            Err(_) => Vec::new(),
        }
    }
}

impl Completer for MyCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        let line = &line[..pos];
        let parts: Vec<&str> = line.split_whitespace().collect();

        // Calculate the current word and its span
        let last_space = line.rfind(' ').map(|i| i + 1).unwrap_or(0);
        let span = Span::new(last_space, pos);
        let current_word = &line[last_space..pos];

        // Complete commands at beginning
        if parts.is_empty() || (parts.len() == 1 && last_space == 0) {
            return self.commands
                .iter()
                .filter(|cmd| cmd.starts_with(current_word))
                .map(|cmd| Suggestion {
                    value: cmd.to_string(),
                    span,
                    append_whitespace: true,
                    ..Default::default()
                })
                .collect();
        }
        
        // Always complete files if path contains '/' or starts with '~'
        if current_word.contains('/') || current_word.starts_with('~') {
            return self.complete_files(current_word, span);
        }
        
        // For first token after command, try subcommands
        if parts.len() == 1 {
            let main_cmd = parts[0];
            let subcommands = self.get_subcommands(main_cmd);
            
            if !subcommands.is_empty() {
                return subcommands
                    .iter()
                    .filter(|subcmd| subcmd.starts_with(current_word))
                    .map(|subcmd| Suggestion {
                        value: subcmd.to_string(),
                        span,
                        append_whitespace: true,
                        ..Default::default()
                    })
                    .collect();
            }
        }
        
        // Otherwise, complete files in current directory
        self.complete_files(current_word, span)
    }
}

/// Expand paths starting with tilde to home directory
fn expand_tilde(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix('~') {
        if let Some(home) = dirs::home_dir() {
            return home.join(stripped.trim_start_matches('/'));
        }
    }
    PathBuf::from(path)
}

/// Create sanitized filename for cache
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

/// Create default completer instance
pub fn create_default_completer() -> Box<dyn Completer> {
    Box::new(MyCompleter::new())
}
