mod config;
mod commands;
mod completion;
mod shell;
mod prompt;
mod utils;
mod validator;

use nu_ansi_term::{Color, Style};
use reedline::{
    default_emacs_keybindings,
    ColumnarMenu,
    DefaultHinter,
    Emacs,
    FileBackedHistory,
    KeyCode,
    KeyModifiers,
    MenuBuilder,
    Reedline,
    ReedlineEvent,
    ReedlineMenu,
    Signal
};

use crate::{
    completion::create_default_completer, 
    prompt::SimplePrompt,
    validator::MyValidator
};

fn main() {
    let config = config::init();
    config::run_startup(&config);

    let history_path = config::history_file_path();
    
    // Create history with file backing
    let history = FileBackedHistory::with_file(1000, history_path)
        .unwrap_or_else(|e| {
            eprintln!("Failed to load history: {}", e);
            FileBackedHistory::default()
        });

    // Create completer
    let completer = create_default_completer();

    let completion_menu = Box::new(
        ColumnarMenu::default()
            .with_name("completion_menu")
            .with_column_width(Some(20))
    );

    let mut keybindings = default_emacs_keybindings();
    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("completion_menu".to_string()),
            ReedlineEvent::MenuNext,
        ]),
    );

    keybindings.add_binding(
        KeyModifiers::SHIFT,
        KeyCode::BackTab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("completion_menu".to_string()),
            ReedlineEvent::MenuPrevious,
        ]),
    );

    let edit_mode = Box::new(Emacs::new(keybindings));
    let validator = Box::new(MyValidator);
    
    // Create command line editor
    let mut line_editor = Reedline::create()
        .with_history(Box::new(history))
        .with_completer(completer)
        .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
        .with_edit_mode(edit_mode)
        .with_validator(validator)
        .with_hinter(Box::new(
            DefaultHinter::default()
                .with_style(Style::new().italic().fg(Color::Rgb(120, 120, 120)))
                .with_min_chars(1)
        ));

    let prompt = SimplePrompt::new(&config);
    
    loop {
        match line_editor.read_line(&prompt) {
            Ok(Signal::Success(buffer)) => {
                let trimmed = buffer.trim();
                if !trimmed.is_empty() {
                    config::append_to_history(trimmed);
                    if let Err(e) = shell::execute(trimmed) {
                        if !e.contains("Command failed with code") {
                            eprintln!("\x1b[31mError: {}\x1b[0m", e);
                        }
                    }
                }
            }
            Ok(Signal::CtrlD) => break,
            Ok(Signal::CtrlC) => continue,
            Err(e) => eprintln!("Readline error: {:?}", e),
        }
    }
}
