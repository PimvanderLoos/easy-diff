//! Saves and restores terminal state on abnormal exit (Ctrl-C, panic).

#[cfg(unix)]
mod unix {
    use std::io::Write;
    use std::sync::OnceLock;

    static ORIGINAL_TERMIOS: OnceLock<libc::termios> = OnceLock::new();

    fn save_terminal_state() {
        unsafe {
            let mut termios = std::mem::MaybeUninit::<libc::termios>::zeroed();
            if libc::tcgetattr(libc::STDIN_FILENO, termios.as_mut_ptr()) == 0 {
                let _ = ORIGINAL_TERMIOS.set(termios.assume_init());
            }
        }
    }

    fn restore_terminal_state() {
        if let Some(termios) = ORIGINAL_TERMIOS.get() {
            unsafe {
                libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, termios);
            }
        }
        let _ = std::io::stderr().write_all(b"\x1b[?25h");
    }

    fn install_signal_handler() {
        unsafe {
            extern "C" fn handler(_sig: libc::c_int) {
                restore_terminal_state();
                unsafe {
                    libc::signal(libc::SIGINT, libc::SIG_DFL);
                    libc::raise(libc::SIGINT);
                }
            }

            let mut sa: libc::sigaction = std::mem::zeroed();
            sa.sa_sigaction = handler as *const () as usize;
            sa.sa_flags = libc::SA_RESETHAND;
            libc::sigemptyset(&mut sa.sa_mask);
            libc::sigaction(libc::SIGINT, &sa, std::ptr::null_mut());
        }
    }

    pub fn install_terminal_guard() {
        save_terminal_state();
        install_signal_handler();

        let original_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            restore_terminal_state();
            original_hook(info);
        }));
    }
}

#[cfg(not(unix))]
mod fallback {
    pub fn install_terminal_guard() {}
}

#[cfg(not(unix))]
pub use fallback::install_terminal_guard;
#[cfg(unix)]
pub use unix::install_terminal_guard;
