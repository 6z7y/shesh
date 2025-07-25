use std::{
    io,ptr,
    ffi::CString,
    process::exit,
    os::fd::{AsFd, AsRawFd}
};

use libc::{
    fork,pipe,STDOUT_FILENO,
    SIG_DFL,setsid,signal,
    SIGTTIN,SIGINT,SIG_IGN,
    SIGTTOU,SIGQUIT,STDIN_FILENO,
    dup2,close,waitpid
};
use crate::{
    parse::{ParsedCommand, Operator},
    shell::run
};

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
        Err(io::Error::other(
            format!("Command failed with status {status}")
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


pub fn run_background(command: ParsedCommand) -> io::Result<()> {
    let pid = unsafe { fork() };
    match pid {
        0 => { // Child process
            unsafe { setsid(); }
            
            // Reset signal handlers
            unsafe {
                signal(SIGINT, SIG_DFL);
                signal(SIGQUIT, SIG_DFL);
                signal(SIGTTOU, SIG_IGN);
                signal(SIGTTIN, SIG_IGN);
            }
            
            // Redirect standard I/O
            let null = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open("/dev/null")?;
            
            unsafe {
                dup2(null.as_raw_fd(), 0);
                dup2(null.as_raw_fd(), 1);
                dup2(null.as_fd().as_raw_fd(), 2);
            }
            
            let _ = run(command);
            std::process::exit(0);
        },
        pid if pid > 0 => {
            println!("[{pid}] Running in background");
            Ok(())
        },
        _ => Err(io::Error::last_os_error())
    }
}
