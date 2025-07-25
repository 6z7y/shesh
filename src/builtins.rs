use std::ffi::CString;
use std::{
    env,
    ptr,
    io,
};
use libc::{fork, execvp, waitpid};

use crate::{
    utils::expand_tilde
};

pub fn cd(args: &[&str]) -> io::Result<()> {
    let dir = args.first().unwrap_or(&"~");
    let path = expand_tilde(dir);
    
    env::set_current_dir(&path).map_err(|e| {
        let msg = format!("cd: {}: {}", path.display(), e);
        io::Error::new(io::ErrorKind::Other, msg)
    })
}

pub fn execute_external(command: &str, args: &[&str]) -> io::Result<()> {
    // Prepare command and args as C strings
    let cmd_cstr = CString::new(command)?;
    let all_args = std::iter::once(command).chain(args.iter().copied());
    
    // Convert all arguments to CStrings
    let args_cstr: Vec<CString> = all_args
        .map(|a| CString::new(a))
        .collect::<Result<_, _>>()?;
    
    // Build argv array (pointers + null terminator)
    let argv: Vec<*const libc::c_char> = args_cstr
        .iter()
        .map(|c| c.as_ptr())
        .chain(std::iter::once(ptr::null()))
        .collect();
    
    unsafe {
        match fork() {
            0 => { // Child process
                execvp(cmd_cstr.as_ptr(), argv.as_ptr());
                libc::exit(1); // Only reached if execvp fails
            },
            -1 => Err(io::Error::last_os_error()), // Fork failed
            pid => { // Parent process
                let mut status = 0;
                // WNOHANG would return immediately if child hasn't exited
                // We use 0 to wait (remove WNOHANG for blocking wait)
                while waitpid(pid, &mut status, 0) > 0 {}
                
                if libc::WIFEXITED(status) && libc::WEXITSTATUS(status) != 0 {
                    Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!("Command failed with status {}", libc::WEXITSTATUS(status)))
                    )
                } else {
                    Ok(())
                }
            }
        }
    }
}

pub fn help()-> String {
    format!("
Available builtins:
- cd [dir] : Change directory
- exit     : Exit the shell
- help     : Show this help
")
}
