use std::env;

use reedline::{Prompt, PromptEditMode, PromptHistorySearch};

pub struct SimplePrompt;

impl SimplePrompt {
    pub fn new() -> Self {
        Self
    }
}

impl Prompt for SimplePrompt {
    fn render_prompt_left(&self) -> std::borrow::Cow<'static, str> {
        let path = env::current_dir().ok().map(|p| p.display().to_string()).unwrap_or("no path".into());
        let homedir = env::var("HOME").unwrap_or_default();
        let new_path = path.replace(&homedir, "~");

        let segments: Vec<&str> = new_path.split('/').filter(|s| !s.is_empty()).collect();
        let len = segments.len();

        let base_prompt = if len == 0 {
            if new_path.starts_with('/') {
                "/> ".to_string()
            } else {
                "> ".to_string()
            }
        } else {
            let start = if new_path.starts_with('/') { "/" } else { "" };

            let shortened = segments
                .iter()
                .enumerate()
                .map(|(i, seg)| {
                    if i == len - 1 {
                        seg.to_string()
                    } else if seg.starts_with('.') {
                        format!(".{}", seg.chars().nth(1).unwrap_or_default())
                    } else {
                        seg.chars().next().unwrap_or_default().to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("/");

            format!("{start}{shortened}> ")
        };

        std::borrow::Cow::Owned(base_prompt)
    }

    fn render_prompt_right(&self) -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("")
    }

    fn render_prompt_indicator(&self, _edit_mode: PromptEditMode) -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("> ")
    }

    fn render_prompt_multiline_indicator(&self) -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("::: ")
    }

    fn render_prompt_history_search_indicator(&self, _history_search: PromptHistorySearch) -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("⭠ ")
    }
}
