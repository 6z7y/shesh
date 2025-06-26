mod core;
mod symbols;

use self::core::*;
use self::symbols::*;

/// Execute a command with support for special symbols
pub fn execute_command(tokens: &[String]) -> Result<(), String> {
    if tokens.is_empty() {
        return Ok(());
    }

    // Try to handle special symbols first
    if let Ok(()) = handle_symbols(tokens) {
        return Ok(());
    }

    // No special symbols found, process as regular command
    let input_str = tokens.join(" ");
    let expanded = expand_aliases(&input_str);
    let expanded_tokens = expanded.split_whitespace()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    if expanded_tokens.is_empty() {
        return Ok(());
    }

    // Execute built-in or external command
    match expanded_tokens[0].as_str() {
        "alias" => handle_alias_cmd(&expanded_tokens[1..]),
        "cd" => cd(&expanded_tokens[1..]),
        "exit" => std::process::exit(0),
        "help" => help(),
        cmd => execute_external_command(cmd, &expanded_tokens[1..]),
    }
}
