//! This crate provides the following three functions:
//!
//! ```rust
//! # trait Ignore {
//! fn stdin_isatty() -> bool
//! # ;
//! fn stdout_isatty() -> bool
//! # ;
//! fn stderr_isatty() -> bool
//! # ;
//! # }
//! ```
//!
//! On Linux and Mac they are implemented with [`libc::isatty`]. On Windows they
//! are implemented with [`kernel32::GetConsoleMode`]. On Redox they are
//! implemented with [`termion::is_tty`].
//!
//! [`libc::isatty`]: http://man7.org/linux/man-pages/man3/isatty.3.html
//! [`kernel32::GetConsoleMode`]: https://msdn.microsoft.com/en-us/library/windows/desktop/ms683167.aspx
//! [`termion::is_tty`]: https://docs.rs/termion/1.5.1/termion/fn.is_tty.html
//!
//! The `stdin_isatty` function is not yet implemented for Windows. If you need
//! it, please check [dtolnay/isatty#1] and contribute an implementation!
//!
//! [dtolnay/isatty#1]: https://github.com/dtolnay/isatty/issues/1
//!
//! ## Usage
//!
//! `Cargo.toml`
//!
//! > ```toml
//! > [dependencies]
//! > isatty = "0.1"
//! > ```
//!
//! `src/main.rs`
//!
//! > ```rust
//! > extern crate isatty;
//! > use isatty::{stdin_isatty, stdout_isatty, stderr_isatty};
//! >
//! > fn main() {
//! >     println!("stdin: {}", stdin_isatty());
//! >     println!("stdout: {}", stdout_isatty());
//! >     println!("stderr: {}", stderr_isatty());
//! > }
//! > ```

#![doc(html_root_url = "https://docs.rs/isatty/0.1.6")]

// Based on:
//  - https://github.com/rust-lang/cargo/blob/099ad28104fe319f493dc42e0c694d468c65767d/src/cargo/lib.rs#L154-L178
//  - https://github.com/BurntSushi/ripgrep/issues/94#issuecomment-261761687

#[cfg(not(windows))]
pub fn stdin_isatty() -> bool {
    isatty(stream::Stream::Stdin)
}

pub fn stdout_isatty() -> bool {
    isatty(stream::Stream::Stdout)
}

pub fn stderr_isatty() -> bool {
    isatty(stream::Stream::Stderr)
}

mod stream {
    pub enum Stream {
        #[cfg(not(windows))]
        Stdin,
        Stdout,
        Stderr,
    }
}

#[cfg(unix)]
use unix::isatty;
#[cfg(unix)]
mod unix {
    use stream::Stream;

    pub fn isatty(stream: Stream) -> bool {
        extern crate libc;

        let fd = match stream {
            Stream::Stdin => libc::STDIN_FILENO,
            Stream::Stdout => libc::STDOUT_FILENO,
            Stream::Stderr => libc::STDERR_FILENO,
        };

        unsafe { libc::isatty(fd) != 0 }
    }
}

#[cfg(windows)]
use windows::isatty;
#[cfg(windows)]
mod windows {
    extern crate kernel32;
    extern crate winapi;

    use stream::Stream;

    pub fn isatty(stream: Stream) -> bool {
        let handle = match stream {
            Stream::Stdout => winapi::winbase::STD_OUTPUT_HANDLE,
            Stream::Stderr => winapi::winbase::STD_ERROR_HANDLE,
        };

        unsafe {
            let handle = kernel32::GetStdHandle(handle);

            // check for msys/cygwin
            if is_cygwin_pty(handle) {
                return true;
            }

            let mut out = 0;
            kernel32::GetConsoleMode(handle, &mut out) != 0
        }
    }

    /// Returns true if there is an MSYS/cygwin tty on the given handle.
    fn is_cygwin_pty(handle: winapi::HANDLE) -> bool {
        use std::ffi::OsString;
        use std::mem;
        use std::os::raw::c_void;
        use std::os::windows::ffi::OsStringExt;
        use std::slice;

        use self::kernel32::GetFileInformationByHandleEx;
        use self::winapi::fileapi::FILE_NAME_INFO;
        use self::winapi::minwinbase::FileNameInfo;
        use self::winapi::minwindef::MAX_PATH;

        unsafe {
            let size = mem::size_of::<FILE_NAME_INFO>();
            let mut name_info_bytes = vec![0u8; size + MAX_PATH];
            let res = GetFileInformationByHandleEx(handle,
                                                FileNameInfo,
                                                &mut *name_info_bytes as *mut _ as *mut c_void,
                                                name_info_bytes.len() as u32);
            if res == 0 {
                return true;
            }
            let name_info: FILE_NAME_INFO = *(name_info_bytes[0..size]
                .as_ptr() as *const FILE_NAME_INFO);
            let name_bytes = &name_info_bytes[size..size + name_info.FileNameLength as usize];
            let name_u16 = slice::from_raw_parts(name_bytes.as_ptr() as *const u16,
                                                name_bytes.len() / 2);
            let name = OsString::from_wide(name_u16)
                .as_os_str()
                .to_string_lossy()
                .into_owned();
            name.contains("msys-") || name.contains("-pty")
        }
    }
}

#[cfg(target_os = "redox")]
use redox::isatty;
#[cfg(target_os = "redox")]
mod redox {
    use stream::Stream;

    pub fn isatty(stream: Stream) -> bool {
        extern crate termion;
        use std::io;

        match stream {
            Stream::Stdin => termion::is_tty(&io::stdin()),
            Stream::Stdout => termion::is_tty(&io::stdout()),
            Stream::Stderr => termion::is_tty(&io::stderr()),
        }
    }
}
