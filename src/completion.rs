use std::{
    collections::{HashMap, HashSet},
    fs::{self, OpenOptions},
    io::{BufReader, BufWriter, BufRead, Write},
    path::PathBuf,
    process::Command
};
use reedline::{Completer, Suggestion, Span};
use home::home_dir;

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
        let cache_dir = home_dir()
            .map(|mut path| {
                path.push(".cache");
                path.push("shesh/completions");
                path
            })
                .unwrap_or_else(|| PathBuf::from("/tmp/shesh/completions"));
        
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
        let builtins = ["cd","exit","help"];
        for b in builtins {
            commands.insert(b.to_string());
        };
        commands
    }

    /// Get path to cache file for a command
    fn get_cache_path(&self, cmd: &str) -> PathBuf {
        self.cache_dir.join(format!("{}.24", sanitize_filename(cmd)))
    }

    /// Get subcommands for a command, using cache when available
    fn get_subcommands(&mut self, cmd: &str) -> Vec<String> {
        if let Some(cached) = self.load_from_cache(cmd) {
            return cached;
        }

        let subcommands = self.extract_subcommands(cmd);
        if !subcommands.is_empty() {
            let _ = self.save_to_cache(cmd, &subcommands);
            self.subcommand_cache.insert(cmd.to_string(), subcommands.clone());
        }

        subcommands
    }

    /// Save subcommands to cache file
    fn save_to_cache(&self, cmd: &str, subcommands: &[String]) -> Result<(), std::io::Error> {
        let path = self.get_cache_path(cmd);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;

        let mut writer = BufWriter::new(file);
        for sub in subcommands {
            writeln!(writer, "{}", sub)?;
        }

        Ok(())
    }

    /// Try to load cached subcommands from disk
    fn load_from_cache(&self, cmd: &str) -> Option<Vec<String>> {
        let cache_file = self.get_cache_path(cmd);
        if !cache_file.exists() {
            return None;
        }

        let file = OpenOptions::new().read(true).open(&cache_file).ok()?;
        let reader = BufReader::new(file);

        let subcommands: Vec<String> = reader
            .lines()
            .map_while(Result::ok)
            .filter(|line| !line.trim().is_empty())
            .collect();

        if subcommands.is_empty() {
            return None;
        }

        Some(subcommands)
    }

    /// Extract subcommands by parsing `cmd --help`
    fn extract_subcommands(&self, cmd: &str) -> Vec<String> {
        let output = match Command::new(cmd).arg("--help").output().ok() {
            Some(output) => output,
            None => return Vec::new(),
        };
        let help = String::from_utf8_lossy(&output.stdout);
        
        let mut subs = Vec::new();

        for line in help.lines() {
            if line.starts_with("  ") {
                if let Some(token) = line.split_whitespace().next() {
                    if token.len() > 1 && !token.contains(['<', '"', '[', '(']) {
                        subs.push(token.trim_end_matches(',').to_string());
                    }
                }
            }
        }
        subs.sort();
        subs.dedup();
        subs
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
// fn expand_tilde(path: &str) -> PathBuf {
//     if let Some(stripped) = path.strip_prefix('~') {
//         if let Some(home) = dirs::home_dir() {
//             return home.join(stripped.trim_start_matches('/'));
//         }
//     }
//     PathBuf::from(path)
// }
fn expand_tilde(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix('~') {
        if let Some(home) = home_dir() {
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
