use reedline::{Prompt, PromptEditMode, PromptHistorySearch};
use std::borrow::Cow;

pub struct SimplePrompt {
    prompt: Cow<'static, str>,
}

impl SimplePrompt {
    pub fn new(config: &crate::config::Config) -> Self {
        let prompt = config.prompt.as_deref().unwrap_or("shesh> ");
        Self {
            prompt: Cow::Owned(prompt.to_string()),
        }
    }
}

impl Prompt for SimplePrompt {
    fn render_prompt_left(&self) -> Cow<'static, str> {
        self.prompt.clone()
    }

    fn render_prompt_right(&self) -> Cow<'static, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_indicator(&self, _mode: PromptEditMode) -> Cow<'static, str> {
        Cow::Borrowed("❯ ")
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'static, str> {
        Cow::Borrowed("     ")
    }

    fn render_prompt_history_search_indicator(&self, _history_search: PromptHistorySearch) -> Cow<'static, str> {
        Cow::Borrowed("⭠ ")
    }
}
