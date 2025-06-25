use reedline::{Prompt, PromptEditMode, PromptHistorySearch};
use std::{borrow::Cow, env};

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
        if let Some(ref prompt) = self.custom_prompt {
            return prompt.clone();
        }

        // Get current dir and replace $HOME with ~
        let path = env::current_dir()
            .ok()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "no path".to_string());

        let homedir = env::var("HOME").unwrap_or_default();
        let new_path = path.replace(&homedir, "~");

        Cow::Owned(format!("{}> ", new_path))
    }

    fn render_prompt_right(&self) -> Cow<'static, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_indicator(&self, _mode: PromptEditMode) -> Cow<'static, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'static, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_history_search_indicator(&self, _history_search: PromptHistorySearch) -> Cow<'static, str> {
        Cow::Borrowed("⭠ ")
    }
}

