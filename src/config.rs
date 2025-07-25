use std::{
    env,io::Write,
    fs::{self, create_dir_all, OpenOptions},
    path::{PathBuf, Path},
    process::exit
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

pub fn get_home() -> PathBuf {
    env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| {
        eprintln!("can't find the home dir");
        exit(1)
    })
}

pub fn get_config() -> PathBuf {
    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| get_home().join(".config"))
}

// pub fn get_cache() -> PathBuf {
//     env::var_os("XDG_CACHE_HOME")
//         .map(PathBuf::from)
//         .unwrap_or_else(|| get_home().join(".cache"))
// }

pub fn config_file_path() -> PathBuf {
    get_config().join("shesh").join("shesh.24")
}


pub fn history_file_path() -> PathBuf {
    get_home().join(".local/share/shesh/history")
}

//config file
pub fn init() -> Config {
    let config_path = config_file_path();

    if let Some(parent) = config_path.parent() {
        let _ = create_dir_all(parent);
    }

    if !config_path.exists() {
        fs::write(
            &config_path,
            "#prompt = \"shesh> \"\n#startup\necho \"shesh ready!\"",
        )
        .unwrap_or_else(|_| {
            eprintln!("Unable to create a config file");
            exit(1)
        });
    }
    load_config(&config_path)
}

pub fn load_config(path: &Path) -> Config {
    parse_config(&fs::read_to_string(path).unwrap_or_else(|_| {
        eprintln!("Unable to load a config file");
        exit(1)
    }))
}

fn parse_config(content: &str) -> Config {
    let mut config = Config::default();
    let mut in_startup = false;

    for linee in content.lines() {
        let line = linee.trim();
        if !line.is_empty() {
            if let Some(stripped) = line.strip_prefix('#') {
                match stripped.trim() {
                    c if c.starts_with("prompt") => config.prompt = None,
                    c if c.eq_ignore_ascii_case("startup") => in_startup = true,
                    _ => {}
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
pub fn append_to_history(command: &str) {
    let path = history_file_path();

    if path.parent().is_some_and(|p| create_dir_all(p).is_err()) {
        eprintln!("[X] Failed to create history directory");
        return;
    }

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
        if let Err(e) = writeln!(file, "{command}") {
            eprintln!("[X] Failed to write to history file: {e}");
        }
    } else {
        eprintln!("[X] Failed to open history file");
    }
}

