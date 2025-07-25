use std::{
    env,io::Write,
    fs::{self, create_dir_all, OpenOptions},
    path::{PathBuf, Path}
};

pub struct Config {
    pub prompt: Option<String>,
    pub startup: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            prompt: Some("#shesh> ".to_string()),
            startup: vec![],
        }
    }
}

//config file
pub fn config_file_path() -> PathBuf {
    PathBuf::from(env::var("HOME").unwrap())
        .join(".config/shesh/shesh.24")
}

pub fn init()->Config{
    let config_path = config_file_path();

    if let Some(parent) = config_path.parent() {
        let _ = create_dir_all(parent);
    }

    if !config_path.exists() {
        let default = "#prompt = \"shesh> \"\n#startup\necho \"shesh ready!\"";
        let _ = fs::write(&config_path, default);
    }
    load_config(&config_path)
}

pub fn load_config(path:&Path)->Config{
    let content = fs::read_to_string(path).unwrap_or_default();
    parse_config(&content)
}

fn parse_config(content: &str) -> Config {
    let mut config = Config::default();
    let mut in_startup = false;

    for line in content.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if let Some(comment) = line.strip_prefix('#') {
            let comment = comment.trim();
            if comment.starts_with("prompt") {
                config.prompt = None;
            } else if comment.eq_ignore_ascii_case("startup") {
                in_startup = true;
            }
            continue;
        }

        if in_startup {
            config.startup.push(line.to_string());
        } else if let Some((key, value)) = line.split_once('=') {
            if key.trim() == "prompt" {
                config.prompt = Some(value.trim().trim_matches('"').to_string());
            }
        }
    }
    config
}

pub fn run_startup(config: &Config) {
    for cmd_line in &config.startup {
        if !cmd_line.trim().is_empty() {
            if let Err(e) = crate::shell::exec(cmd_line) {
                eprintln!("[X] Startup failed: {e}");
            }
        }
    }
}

//history file
pub fn history_file_path() -> PathBuf {
    PathBuf::from(env::var("HOME").unwrap())
        .join(".local/share/shesh/history")
}

pub fn append_to_history(command: &str) {
    let path = history_file_path();

    if let Some(parent) = path.parent() {
        if create_dir_all(parent).is_err() {
            eprintln!("Failed to create directory: {}", parent.display());
        }
    }

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
        if writeln!(file, "{command}").is_err() {
            eprintln!("Failed to write to: {}", path.display());
        }
    } else {
        eprintln!("Failed to open: {}", path.display());
    }
}
