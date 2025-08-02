use std::{
    env,
    path::PathBuf
};

pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix('~') {
        if let Ok(home) = env::var("HOME") {
            if stripped.is_empty() {
                return PathBuf::from(home);
            } else if let Some(rest) = stripped.strip_prefix('/') {
                return PathBuf::from(home).join(rest);
            }
        }
    }
    PathBuf::from(path)
}

pub fn expand_env_vars(input: &str) -> String {
    let mut result = input.to_string();
    for (key, value) in env::vars() {
        result = result.replace(&format!("${key}"), &value);
    }
    result
}
