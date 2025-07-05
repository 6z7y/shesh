mod b_mod;
mod b_core;
mod b_symbols;
mod completions;
mod config;
mod prompt;
mod shell;
mod utils;
mod validator;use nu_ansi_term::{Color, Style};

use reedline::{
    default_emacs_keybindings, default_vi_insert_keybindings,
    default_vi_normal_keybindings, ColumnarMenu, DefaultHinter,
    Emacs, FileBackedHistory, KeyCode, KeyModifiers, MenuBuilder,
    Reedline, ReedlineEvent, ReedlineMenu, Signal, Vi,
};

use crate::{
    completions::create_default_completer, 
    prompt::SimplePrompt,
    utils::vim_enabled,
    validator::MyValidator
};

fn main() {
    // 1. Initialize configuration
    let config = config::init();
    config::run_startup(&config);
    
    // 2. Create a history loader function
    let load_history = || {
        let history_path = config::history_file_path();
        FileBackedHistory::with_file(6000, history_path)
            .unwrap_or_else(|e| {
                eprintln!("Failed to load history: {}", e);
                FileBackedHistory::default()
            })
    };

    // 3. Initialize auto-completion
    let create_completer = || create_default_completer();
    
    // 4. Create completion menu
    let create_menu = || {
        let completion_menu = Box::new(
            ColumnarMenu::default()
                .with_name("completion_menu")
                .with_column_width(Some(20))
        );
        ReedlineMenu::EngineCompleter(completion_menu)
    };

    // 5. Create editor
    let create_editor = || {
        let history = Box::new(load_history());
        let completer = create_completer();
        let menu = create_menu();
        
        if vim_enabled() {
            // Vim mode
            let vi_insert_keybindings = default_vi_insert_keybindings();
            let vi_normal_keybindings = default_vi_normal_keybindings();
            let vi_mode = Vi::new(vi_insert_keybindings, vi_normal_keybindings);
            
            Reedline::create()
                .with_history(history)
                .with_completer(completer)
                .with_edit_mode(Box::new(vi_mode))
                .with_menu(menu)
                .with_validator(Box::new(MyValidator))
                .with_hinter(Box::new(
                    DefaultHinter::default()
                        .with_style(Style::new().italic().fg(Color::Rgb(120, 120, 120)))
                        .with_min_chars(1)
                ))
        } else {
            // Emacs mode
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

            Reedline::create()
                .with_history(history)
                .with_completer(completer)
                .with_menu(menu)
                .with_edit_mode(Box::new(Emacs::new(keybindings)))
                .with_validator(Box::new(MyValidator))
                .with_hinter(Box::new(
                    DefaultHinter::default()
                        .with_style(Style::new().italic().fg(Color::Rgb(120, 120, 120)))
                        .with_min_chars(1)
                ))
        }
    };
    
    let mut line_editor = create_editor();
    let prompt = SimplePrompt::new(&config);
    
    // 6. Main read-execute loop
    let mut prev_vim = vim_enabled();
    
    loop {
        match line_editor.read_line(&prompt) {
            Ok(Signal::Success(buffer)) => {
                let trimmed = buffer.trim();
                if !trimmed.is_empty() {
                    // Add command to history
                    config::append_to_history(trimmed);
                    
                    // Execute command
                    if let Err(e) = shell::execute(trimmed) {
                        if !e.contains("Command failed with code") {
                            eprintln!("\x1b[31mError: {}\x1b[0m", e);
                        }
                    }
                    
                    // Check if vim mode changed
                    let current_vim = vim_enabled();
                    if current_vim != prev_vim {
                        line_editor = create_editor();
                        prev_vim = current_vim;
                    }
                }
            }
            Ok(Signal::CtrlD) => break,
            Ok(Signal::CtrlC) => continue,
            Err(e) => eprintln!("Readline error: {:?}", e),
        }
    }
}
