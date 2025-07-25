use std::{
    env,
    fs
};

// Define supported shell operators
#[derive(Debug, Clone, PartialEq)]
pub enum Operator {
    And,         // && (logical AND)
    Or,          // || (logical OR)
    Pipe,        // | (pipe)
    Seq,         // ; (sequential)
    Background,  // & (background process)
}

// AST (Abstract Syntax Tree) representation of commands
#[derive(Debug, Clone)]
pub enum ParsedCommand {
    Single(Vec<String>),  // Simple command (e.g., "ls -l")
    BinaryOp(Box<ParsedCommand>, Operator, Box<ParsedCommand>),  // Compound command with operator
}

// Main parsing function - entry point
pub fn parse_syntax(input: &str) -> ParsedCommand {
    parse_logical_ops(input.trim())  // Trim whitespace and start parsing
}

// Recursive function to parse logical operators (&&, ||, etc.)
fn parse_logical_ops(input: &str) -> ParsedCommand {
    // Operators ordered by precedence (lower precedence first)
    let ops = [
        (";", Operator::Seq),        // Lowest precedence
        ("&&", Operator::And),
        ("||", Operator::Or),
        ("|", Operator::Pipe),
        ("&", Operator::Background),  // Highest precedence
    ];

    // Check for each operator in the input string
    for (op_str, op_enum) in ops.iter() {
        // Find operator that's not inside quotes
        if let Some(index) = find_outside_quotes(input, op_str) {
            // Split command at the operator
            let (left, right) = input.split_at(index);
            let right = &right[op_str.len()..];
            
            // Recursively parse both sides
            return ParsedCommand::BinaryOp(
                Box::new(parse_logical_ops(left)),
                op_enum.clone(),
                Box::new(parse_logical_ops(right)),
            );
        }
    }

    // If no operators found, treat as simple command
    ParsedCommand::Single(tokenize(input))
}

// Finds operator occurrences outside quoted strings
fn find_outside_quotes(input: &str, target: &str) -> Option<usize> {
    let mut in_single = false;  // Inside single quotes
    let mut in_double = false;  // Inside double quotes
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i + target.len() <= chars.len() {
        // Toggle quote tracking
        match chars[i] {
            '"' if !in_single => in_double = !in_double,
            '\'' if !in_double => in_single = !in_single,
            _ => {}
        }

        // Only match if not inside quotes
        if !in_single && !in_double && input[i..].starts_with(target) {
            return Some(i);
        }

        i += 1;
    }

    None  // Operator not found
}

// Splits command into tokens while respecting quotes
fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();
    let mut in_single = false;
    let mut in_double = false;

    while let Some(c) = chars.next() {
        match c {
            // Handle quotes
            '"' if !in_single => in_double = !in_double,
            '\'' if !in_double => in_single = !in_single,
            
            // Split on space only when not in quotes
            ' ' if !in_single && !in_double => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            }
            
            // Add character to current token
            _ => current.push(c),
        }
    }

    // Add the last token if exists
    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

// Processes tokens by expanding variables and wildcards
pub fn process_tokens(cmd: ParsedCommand) -> Vec<String> {
    match cmd {
        ParsedCommand::Single(parts) => {
            let mut result = Vec::new();
            for part in parts {
                if part.starts_with('$') {
                    // Expand environment variable
                    let var = &part[1..];
                    let val = env::var(var).unwrap_or_default();
                    result.push(val);
                } 
                else if part == "*" {
                    // Expand wildcard to list files in current directory
                    if let Ok(entries) = fs::read_dir(".") {
                        for entry in entries.flatten() {
                            result.push(entry.file_name().to_string_lossy().into());
                        }
                    }
                }
                else if part.starts_with('~') {
                    // Expand home directory shortcut
                    if let Some(home) = env::var_os("HOME") {
                        let home_str = home.to_string_lossy();
                        let rest = &part[1..];
                        result.push(format!("{home_str}{rest}"));
                    } else {
                        result.push(part); // Keep original if HOME isn't set
                    }
                }
                else if part.contains('{') && part.contains('}') {
                    // Expand brace patterns (e.g., file{1,2,3}.txt)
                    for expanded in expand_braces(&part) {
                        result.push(expanded);
                    }
                }
                else {
                    // No expansion needed - use original token
                    result.push(part);
                }
            }
            result
        }
        _ => vec!["[complex command not handled yet]".into()],
    }
}

// {}
fn expand_braces(input: &str) -> Vec<String> {
    if let Some(start) = input.find('{') {
        if let Some(end) = input[start..].find('}') {
            let end = start + end;
            let before = &input[..start];
            let after = &input[end + 1..];
            let inside = &input[start + 1..end];
            return inside
                .split(',')
                .flat_map(|opt| expand_braces(&format!("{before}{opt}{after}")))
                .collect();
        }
    }
    vec![input.to_string()]
}
