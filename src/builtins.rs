use std::{
    env,
    ptr,
    io,
    ffi::CString,
};
use libc::{dup2, fork, execvp, waitpid};

use crate::{
    utils::expand_tilde
};

pub fn cd(args: &[&str]) -> io::Result<()> {
    let dir = args.first().unwrap_or(&"~");
    let path = expand_tilde(dir);
    
    env::set_current_dir(&path).map_err(|e| {
        let msg = format!("cd: {}: {e}", path.display());
        io::Error::other(msg)
    })
}

pub fn help()-> String {
    "
    Available builtins:
    - cd [dir] : Change directory
    - exit     : Exit the shell
    - help     : Show this help
    ".to_string()
}

pub fn execute_external(command: &str, args: &[&str]) -> io::Result<()> {
    // Prepare command and args as C strings
    let cmd_cstr = CString::new(command)?;
    let all_args = std::iter::once(command).chain(args.iter().copied());
    
    // Convert all arguments to CStrings
    let args_cstr: Vec<CString> = all_args
        .map(|s| CString::new(s).map_err(|e| io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Invalid argument: {e}")
        )))
        .collect::<Result<_, _>>()?;
    
    // Build argv array
    let argv: Vec<*const libc::c_char> = args_cstr
        .iter()
        .map(|c| c.as_ptr())
        .chain(std::iter::once(ptr::null()))
        .collect();
    
    unsafe {
        match fork() {
            0 => { // Child process
                libc::signal(libc::SIGINT, libc::SIG_DFL);
                libc::signal(libc::SIGQUIT, libc::SIG_DFL);

                // Redirect stderr to stdout to capture command's own error messages
                dup2(libc::STDOUT_FILENO, libc::STDERR_FILENO);
                
                execvp(cmd_cstr.as_ptr(), argv.as_ptr());
                // Only reached if execvp fails
                libc::exit(127); // Standard "not found" exit code
            },
            -1 => Err(io::Error::last_os_error()), // Fork failed
            pid => { // Parent process
                let mut status = 0;
                waitpid(pid, &mut status, 0);
                
                if libc::WIFEXITED(status) {
                    match libc::WEXITSTATUS(status) {
                        0 => Ok(()),
                        127 => Err(io::Error::new(
                            io::ErrorKind::NotFound,
                            format!("'{command}': isn't installed.")
                        )),
                        _ => Ok(()) // Ignore other exit codes (commands handle their own errors)
                    }
                } else {
                    // Only report signals if they're not part of normal operation
                    if libc::WIFSIGNALED(status) {
                        let sig = libc::WTERMSIG(status);
                        if sig != libc::SIGINT && sig != libc::SIGTERM {
                            return Err(io::Error::new(
                                io::ErrorKind::Interrupted,
                                format!("Command terminated by signal {sig}")
                            ));
                        }
                    }
                    Ok(())
                }
            }
        }
    }
}
