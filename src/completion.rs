use std::{
    collections::{HashMap, HashSet},
    env,
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Write},
    path::PathBuf,
    process::Command
};
use reedline::{Completer, Suggestion, Span};
use crate::utils::expand_tilde;

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
        let cache_dir = PathBuf::from(env::var("HOME").unwrap())
            .join(".cache/shesh/completions");
        
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

        if let Some(path_var) = env::var_os("PATH") {
            env::split_paths(&path_var)
                .flat_map(|dir| fs::read_dir(dir).ok().into_iter().flatten())
                .filter_map(|entry| entry.ok().and_then(|e| e.file_name().to_str().map(str::to_string)))
                .for_each(|cmd| {
                    commands.insert(cmd);
                });
        }

        // Add built-in commands
        let builtins = ["alias","cd","exit","help"];
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
            None
        } else {
            Some(subcommands)
        }
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

    /// Handle file/directory completions without using `match`
    fn complete_files(&self, current: &str, span: Span) -> Vec<Suggestion> {
        let last_slash = current.rfind('/').map_or(0, |i| i + 1);
        let (base, partial) = current.split_at(last_slash);

        let expanded_base = if base.is_empty() {
            PathBuf::from(".")
        } else {
            expand_tilde(base).unwrap_or_else(|_| PathBuf::from(base))
        };

        if !expanded_base.is_dir() {
            return Vec::new();
        }

        let partial_span = Span::new(span.start + last_slash, span.end);

        let reader = match fs::read_dir(&expanded_base) {
            Ok(rd) => rd,
            Err(_) => return Vec::new(),
        };

        reader
            .flatten()
            .filter_map(|entry| {
                // Fix: Create a binding for file_name
                let file_name = entry.file_name();
                let name = file_name.to_str()?;
                
                if !partial.starts_with('.') && name.starts_with('.') {
                    return None;
                }
                
                if !name.starts_with(partial) {
                    return None;
                }

                let value = if entry.path().is_dir() {
                    format!("{}/", name)
                } else {
                    name.to_string()
                };
                
                Some(Suggestion {
                    value,
                    span: partial_span,
                    ..Default::default()
                })
            })
        .collect()
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
