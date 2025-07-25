use libc::{self, fork, pipe, dup2, close, STDIN_FILENO, STDOUT_FILENO, waitpid};
use std::{
    io,
    ffi::CString,
    process::exit,
    ptr
};

use crate::parse::{ParsedCommand, Operator};

/// Unified pipe and command execution
pub fn run_pipe(commands: Vec<ParsedCommand>) -> io::Result<()> {
    if commands.len() < 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Pipe requires at least 2 commands"
        ));
    }

    let mut prev_read = None;
    let mut child_pids = Vec::new();

    for (i, cmd) in commands.iter().enumerate() {
        let is_last = i == commands.len() - 1;
        let mut fds = [0; 2];

        if !is_last && unsafe { pipe(fds.as_mut_ptr()) } == -1 {
            return Err(io::Error::last_os_error());
        }

        match unsafe { fork() } {
            0 => { // Child process
                if let Some(fd) = prev_read {
                    unsafe { dup2(fd, STDIN_FILENO); close(fd); }
                }

                if !is_last {
                    unsafe { dup2(fds[1], STDOUT_FILENO); close(fds[1]); close(fds[0]); }
                }

                if let ParsedCommand::Single(args) = cmd {
                    let cmd = CString::new(args[0].clone())?;
                    let args: Vec<CString> = args[1..].iter()
                        .map(|s| CString::new(s.as_str()))
                        .collect::<Result<_, _>>()?;
                    
                    let argv: Vec<*const libc::c_char> = std::iter::once(cmd.as_ptr())
                        .chain(args.iter().map(|a| a.as_ptr()))
                        .chain(std::iter::once(ptr::null()))
                        .collect();

                    unsafe {
                        libc::execvp(cmd.as_ptr(), argv.as_ptr());
                        eprintln!("Failed to execute {:?}: {}", args[0], io::Error::last_os_error());
                    }
                }
                exit(1);
            },
            pid if pid > 0 => { // Parent
                if let Some(fd) = prev_read {
                    unsafe { close(fd); }
                }
                if !is_last {
                    unsafe { close(fds[1]); }
                    prev_read = Some(fds[0]);
                }
                child_pids.push(pid);
            },
            _ => return Err(io::Error::last_os_error())
        }
    }

    // Wait for all children
    let mut status = 0;
    for pid in child_pids {
        unsafe { waitpid(pid, &mut status, 0); }
    }

    if status != 0 {
        Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Command failed with status {}", status)
        ))
    } else {
        Ok(())
    }
}

/// Flatten nested pipe commands
pub fn flatten_pipes(commands: Vec<ParsedCommand>) -> Vec<ParsedCommand> {
    commands.into_iter().flat_map(|cmd| match cmd {
        ParsedCommand::BinaryOp(left, Operator::Pipe, right) => {
            let mut res = flatten_pipes(vec![*left]);
            res.extend(flatten_pipes(vec![*right]));
            res
        },
        other => vec![other],
    }).collect()
}
