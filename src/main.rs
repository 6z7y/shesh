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
    default_emacs_keybindings, ColumnarMenu, DefaultHinter, Emacs, FileBackedHistory,
    MenuBuilder, KeyCode, KeyModifiers, Reedline, ReedlineEvent, ReedlineMenu, Signal
};

use crate::{
    completions::create_default_completer,
    prompt::SimplePrompt,
};

fn main() {
    // [1] Load configuration and run startup script
    let cfg = config::init();
    config::run_startup(&cfg);

    // [2] Initialize prompt style
    let prompt = SimplePrompt::new();

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

    // [6] Bind Tab key to trigger and navigate completion menu
    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("completion_menu".into()),
            ReedlineEvent::MenuNext,
        ]),
    );

    // [7] Bind Shift+Tab to navigate backward in completion menu
    keybindings.add_binding(
        KeyModifiers::SHIFT,
        KeyCode::BackTab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("completion_menu".into()),
            ReedlineEvent::MenuPrevious,
        ]),
    );

    // [8] Build the line editor with Emacs keybindings, history, completer, and hinter
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
        // تجاهل Ctrl+C في الشيل الرئيسي
        libc::signal(libc::SIGINT, libc::SIG_IGN);
        // تجاهل Ctrl+\
        libc::signal(libc::SIGQUIT, libc::SIG_IGN);
    }

    // [9] Main REPL loop
    loop {
        match editor.read_line(&prompt) {
            Ok(Signal::Success(buf)) if !buf.trim().is_empty() => {
                config::append_to_history(&buf);
                if let Err(e) = shell::exec(&buf) {
                    eprintln!("\x1b[31m- {e}\x1b[0m");
                }
            }
            Ok(Signal::CtrlD) => break,
            Ok(Signal::CtrlC) => continue,
            _ => eprintln!("Reedline error"),
        }
    }
}
