use crate::b_core::{
    expand_aliases, handle_alias_cmd, cd, handle_export_cmd,
    help, execute_external_command, handle_24_command
};
use crate::b_symbols::{handle_symbols, expand_tokens};

pub fn execute_command(tokens: &[String]) -> Result<(), String> {
    if tokens.is_empty() {
        return Ok(());
    }

    // معالجة أمر 24> أولاً قبل أي توسيع
    if tokens[0] == "24>" {
        return handle_24_command(&tokens[1..]);
    }

    // معالجة الرموز الخاصة
    if let Ok(()) = handle_symbols(tokens) {
        return Ok(());
    }

    let input_str = tokens.join(" ");
    let expanded = expand_aliases(&input_str);
    let expanded_tokens = expanded.split_whitespace()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    if expanded_tokens.is_empty() {
        return Ok(());
    }

    let expanded_tokens = expand_tokens(&expanded_tokens);

    match expanded_tokens[0].as_str() {
        "alias" => handle_alias_cmd(&expanded_tokens[1..]),
        "cd" => cd(&expanded_tokens[1..]),
        "export" => handle_export_cmd(&expanded_tokens[1..]),
        "exit" => std::process::exit(0),
        "help" => help(),
        cmd => execute_external_command(cmd, &expanded_tokens[1..]),
    }
}
