use crate::sbi::console_putchar;

use core::fmt::{self, Display, Write};

#[allow(unused)]
#[derive(PartialEq, PartialOrd)]
pub enum LogLevel {
    NONE,
    ERROR,
    WARN,
    INFO,
    TRACE,
    DEBUG,
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "\x1b[{}",
            match self {
                LogLevel::TRACE => "\x1b[90m[TRACE]",
                LogLevel::DEBUG => "\x1b[32m[DEBUG]",
                LogLevel::INFO => "\x1b[34m[INFO]",
                LogLevel::WARN => "\x1b[93m[WARN]",
                LogLevel::ERROR => "\x1b[31m[ERROR]",
                LogLevel::NONE => "\x1b[0m",
            }
        )
    }
}

struct Stdout;

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            console_putchar(c as usize)
        }
        Ok(())
    }
}

pub fn _print(args: fmt::Arguments) {
    Stdout.write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($($arg: tt)*) => ($crate::console::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::console::_print!("\r\n"));
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::_print(format_args!(concat!($fmt, "\r\n") $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! error {
    ($($arg: tt)*) => {
        #[cfg(feature = "log-error")]
        $crate::println!("{}{}{}", $crate::console::LogLevel::ERROR, format_args!($($arg)*), $crate::console::LogLevel::NONE);
    };
}

#[macro_export]
macro_rules! warn {
    ($($arg: tt)*) => {
        #[cfg(feature = "log-warn")]
        $crate::println!("{}{}{}", $crate::console::LogLevel::WARN, format_args!($($arg)*), $crate::console::LogLevel::NONE);
    };
}

#[macro_export]
macro_rules! info {
    ($($arg: tt)*) => {
        #[cfg( feature = "log-info")]
        $crate::println!("{}{}{}", $crate::console::LogLevel::INFO, format_args!($($arg)*), $crate::console::LogLevel::NONE);
    };
}

#[macro_export]
macro_rules! debug {
    ($($arg: tt)*) => {
        #[cfg(feature = "log-debug")]
        $crate::println!("{}{}{}", $crate::console::LogLevel::DEBUG, format_args!($($arg)*), $crate::console::LogLevel::NONE);
    };
}

#[macro_export]
macro_rules! trace {
    ($($arg: tt)*) => {
        #[cfg(feature = "log-trace")]
        $crate::println!("{}{}{}", $crate::console::LogLevel::TRACE, format_args!($($arg)*), $crate::console::LogLevel::NONE);
    };
}
