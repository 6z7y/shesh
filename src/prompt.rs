use std::{borrow::Cow, env};

use reedline::{Prompt, PromptEditMode, PromptHistorySearch, PromptViMode};
use crate::utils::expand;

pub struct SimplePrompt {
    custom_prompt: Option<Cow<'static, str>>,
}

impl SimplePrompt {
    pub fn new(config: &crate::config::Config) -> Self {
        let custom_prompt = config
            .prompt
            .as_ref()
            .map(|s| Cow::Owned(s.to_string()));

        Self { custom_prompt }
    }
}

impl Prompt for SimplePrompt {
    fn render_prompt_left(&self) -> Cow<'static, str> {
        let base_prompt = if let Some(ref prompt) = self.custom_prompt {
            match expand(prompt) {
                Ok(expanded) => expanded,
                Err(_) => prompt.to_string(),
            }
        } else {
            let path = env::current_dir().ok().map(|p| p.display().to_string()).unwrap_or("no path".into());
            let homedir = env::var("HOME").unwrap_or_default();
            let new_path = path.replace(&homedir, "~");

            let segments: Vec<&str> = new_path.split('/').filter(|s| !s.is_empty()).collect();
            let len = segments.len();

            if len == 0 {
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

                format!("{}{}> ", start, shortened)
            }
        };

        Cow::Owned(base_prompt)
    }

    fn render_prompt_right(&self) -> Cow<'static, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_indicator(&self, edit_mode: PromptEditMode) -> Cow<'static, str> {
        match edit_mode {
            PromptEditMode::Vi(vi_mode) => match vi_mode {
                PromptViMode::Normal => Cow::Borrowed("[N] "),
                PromptViMode::Insert => Cow::Borrowed("[I] "),
            },
            _ => Cow::Borrowed("> "),
        }
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'static, str> {
        Cow::Borrowed("::: ")
    }

    fn render_prompt_history_search_indicator(&self, _history_search: PromptHistorySearch) -> Cow<'static, str> {
        Cow::Borrowed("⭠ ")
    }
}
