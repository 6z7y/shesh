use std::{
    io,
};
use crate::{
    builtins::{cd, execute_external, expand_aliases, handle_alias, handle_export_cmd, help},
    parse::{parse_syntax, process_tokens, Operator, ParsedCommand},
    process_exec::{flatten_pipes, run_background, run_pipe,handle_redirect}
};

// Main execution entry point
pub fn exec(cmd: &str) -> io::Result<()> {
    // Check alias command before
    let expanded_cmd = expand_aliases(cmd);
    // Step 1: Parse input string into command structure
    let command = parse_syntax(&expanded_cmd);

    // Step 2: Execute the parsed command
    run(command)
}

// Executes commands based on their parsed structure
pub fn run(cmd: ParsedCommand) -> io::Result<()> {
    match cmd {
        ParsedCommand::Single(args) => {
            if args.is_empty() {
                return Ok(());
            }

            let str_args: Vec<String> = process_tokens(ParsedCommand::Single(args));
            let cmd = str_args[0].as_str();
            let rest: Vec<&str> = str_args[1..].iter().map(|s| s.as_str()).collect();

            match cmd {
                "alias" => handle_alias(&str_args[1..].join(" ")),
                "cd" => cd(&rest),
                "exit" => std::process::exit(0),
                "export" => {
                    let rest_str: Vec<String> = rest.iter().map(|&s| s.to_string()).collect();
                    handle_export_cmd(&rest_str)
                },
                "help" => {
                    println!("{}", help());
                    Ok(())
                },
                _ => execute_external(cmd, &rest)
            }
        }

        // Compound commands with operators (e.g., "cmd1 && cmd2")
        ParsedCommand::BinaryOp(left, op, right) => {
            match op {
                // Sequential execution (;)
                Operator::Seq => {
                    // Execute left command, then right regardless of result
                    run(*left)?;
                    run(*right)
                }
                // Logical AND (&&)
                Operator::And => {
                    // Only execute right if left succeeds
                    if run(*left).is_ok() {
                        run(*right)
                    } else {
                        Ok(())
                    }
                }
                // Logical OR (||)
                Operator::Or => {
                    // Only execute right if left fails
                    if run(*left).is_err() {
                        run(*right)
                    } else {
                        Ok(())
                    }
                }
                Operator::Pipe => {
                    let commands = flatten_pipes(vec![*left, *right]);
                    run_pipe(commands)
                }
                Operator::Background => run_background(*left),
                Operator::Redirect(redirect_type) => handle_redirect(*left, redirect_type, *right),
            }
        }
    }
}
