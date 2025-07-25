use std::{
    io,
};
use libc::{fork};
use crate::{
    parse::{parse_syntax,process_tokens,ParsedCommand,Operator},
    builtins::{cd,execute_external,help},
    process_exec::{run_pipe,flatten_pipes}
};

// Main execution entry point
pub fn exec(cmd: &str) -> io::Result<()> {
    // Step 1: Parse input string into command structure
    let command = parse_syntax(cmd);
    
    // Step 2: Execute the parsed command
    run_command(command)
}

// Executes commands based on their parsed structure
fn run_command(cmd: ParsedCommand) -> io::Result<()> {
    match cmd {
        // Simple command case (e.g., "ls -l")
        ParsedCommand::Single(args) => {
            if args.is_empty() {
                return Ok(());
            }
            // Step 1: Process tokens (expand variables, wildcards)
            let str_args: Vec<String> = process_tokens(ParsedCommand::Single(args));
            
            // Step 2: Separate command name and arguments
            let cmd = str_args[0].as_str();
            let rest: Vec<&str> = str_args[1..].iter().map(|s| s.as_str()).collect();

            // Step 3: Execute based on command type
            match cmd {
                // Built-in commands
                "cd" => cd(&rest),  // Change directory
                "exit" => std::process::exit(0),  // Exit shell
                "help" => {  // Show help
                    println!("{}", help());
                    Ok(())
                }
                // External commands
                _ => execute_external(cmd, &rest),
            }
        }

        // Compound commands with operators (e.g., "cmd1 && cmd2")
        ParsedCommand::BinaryOp(left, op, right) => {
            match op {
                // Sequential execution (;)
                Operator::Seq => {
                    // Execute left command, then right regardless of result
                    run_command(*left)?;
                    run_command(*right)
                }
                // Logical AND (&&)
                Operator::And => {
                    // Only execute right if left succeeds
                    if run_command(*left).is_ok() {
                        run_command(*right)
                    } else {
                        Ok(())
                    }
                }
                // Logical OR (||)
                Operator::Or => {
                    // Only execute right if left fails
                    if run_command(*left).is_err() {
                        run_command(*right)
                    } else {
                        Ok(())
                    }
                }
                Operator::Pipe => {
                    let commands = flatten_pipes(vec![*left, *right]);
                    run_pipe(commands)
                }
                Operator::Background => {
                    let pid = unsafe { fork() };
                    match pid {
                        0 => { // Child process
                            // Reset signal handlers in child
                            unsafe {
                                libc::signal(libc::SIGINT, libc::SIG_DFL);
                                libc::signal(libc::SIGQUIT, libc::SIG_DFL);
                            }
                            let _ = run_command(*left); // Ignore result in background
                            unsafe { libc::exit(0); } // <-- Add unsafe block here
                        },
                        pid if pid > 0 => {
                            println!("Started in background (pid: {pid})");
                            // Detach from child process
                            unsafe { libc::signal(libc::SIGCHLD, libc::SIG_IGN); }
                            Ok(())
                        },
                        _ => Err(io::Error::last_os_error())
                    }
                }
            }
        }
    }
}
