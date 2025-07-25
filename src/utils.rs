use std::{
    env,
    path::PathBuf
};

pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix('~') {
        if stripped.is_empty() {
            return env::var("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("~"));
        }

        // for ~/path or ~/path/subpath
        if let Some(sub_path) = stripped.strip_prefix('/') {
            if let Ok(home) = env::var("HOME") {
                return PathBuf::from(home).join(sub_path);
            }
        }
    }
    PathBuf::from(path)
}
