mod builtins;
mod completions;
mod config;
mod parse;
mod process_exec;
mod prompt;
mod shell;
mod utils;

use nu_ansi_term::{Color, Style};
use reedline::{
    default_emacs_keybindings, ColumnarMenu, DefaultHinter, Emacs, Vi, 
    FileBackedHistory, KeyCode, KeyModifiers, MenuBuilder, 
    Reedline, ReedlineEvent, ReedlineMenu, Signal, EditCommand
};

use crate::{
    completions::create_default_completer,
    prompt::PromptSystem,
};

fn main() {
    // Initialize VIM_MODE
    builtins::init_vim_mode();
    
    // [1] Load configuration and run startup script
    let cfg = config::init();
    config::run_startup(&cfg);

    // [2] Initialize prompt style
    let prompt = PromptSystem::new(cfg.prompt.clone());

    // [3] Set up command history with file persistence
    let history = Box::new(
        FileBackedHistory::with_file(6000, config::history_file_path())
            .unwrap_or_else(|_| FileBackedHistory::default()),
    );

    // [4] Set up auto-completion
    let completer = create_default_completer();

    let menu = ReedlineMenu::EngineCompleter(Box::new(
        ColumnarMenu::default()
            .with_name("completion_menu")
            .with_column_width(Some(20)),
    ));

    // [5] Configure keybindings for Emacs mode
    let mut keybindings = default_emacs_keybindings();
    keybindings.add_binding(
        KeyModifiers::CONTROL,
        KeyCode::Char('c'),
        ReedlineEvent::Edit(vec![EditCommand::Clear]),
    );
    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("completion_menu".into()),
            ReedlineEvent::MenuNext,
        ]),
    );
    keybindings.add_binding(
        KeyModifiers::SHIFT,
        KeyCode::BackTab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("completion_menu".into()),
            ReedlineEvent::MenuPrevious,
        ]),
    );

    // [6] Build the line editor
    let mut editor = Reedline::create()
        .with_history(history)
        .with_completer(completer)
        .with_menu(menu)
        .with_hinter(Box::new(
            DefaultHinter::default()
                .with_style(Style::new().underline().italic().fg(Color::Rgb(120, 120, 120)))
                .with_min_chars(1),
        ))
        .with_edit_mode(Box::new(Emacs::new(keybindings)));

    unsafe {
        libc::signal(libc::SIGINT, libc::SIG_IGN);
        libc::signal(libc::SIGQUIT, libc::SIG_IGN);
    }

    // [7] Main REPL loop
    loop {
        match editor.read_line(&prompt) {
            Ok(Signal::Success(buf)) if !buf.trim().is_empty() => {
                config::append_to_history(&buf);

                if buf.trim() == "24! vim_keys" {
                    let enabled = builtins::toggle_vim_mode();
                    println!("Vim keys {}", if enabled { "enabled" } else { "disabled" });
                    
                    editor = editor.with_edit_mode(if enabled {
                        Box::new(Vi::default())
                    } else {
                        Box::new(Emacs::new(default_emacs_keybindings()))
                    });
                }

                if let Err(e) = shell::exec(&buf) {
                    eprintln!("{e}");
                }
            }
            Ok(Signal::CtrlD) => break,
            Ok(Signal::Success(_)) => continue,
            _ => eprintln!("Reedline error"),
        }
    }
}
