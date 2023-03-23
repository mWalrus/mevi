#[derive(PartialEq, PartialOrd)]
pub(crate) enum LogType {
    Event,
    Info,
    Error,
}

macro_rules! __mevi_dbg {
    ($type:expr, $($arg:tt)*) => {{
        use ::colored::Colorize;
        use ::chrono::Local;
        match $type {
            $crate::LogType::Event if $crate::CLI.debug => {
                println!(
                    "{:?} {}: {}",
                    Local::now(),
                    "[Event]".bold().blue(),
                    format_args!($($arg)*)
                )
            },
            $crate::LogType::Info if $crate::CLI.debug => {
                println!(
                    "{:?} {}: {}",
                    Local::now(),
                    "[Info]".bold().green(),
                    format_args!($($arg)*)
                )
            }
            $crate::LogType::Error => {
                println!(
                    "{:?} {}: {}",
                    Local::now(),
                    "[Error]".bold().red(),
                    format_args!($($arg)*)
                )
            }
            _ => {}
        };
    }};
}

macro_rules! event {
    ($arg:expr) => {{
        use x11rb::protocol::Event;
        match $arg {
            Event::MotionNotify(_) | Event::Error(_) => {}
            _ => __mevi_dbg!($crate::LogType::Event, "{:?}", $arg),
        }
    }};
}

macro_rules! info {
    ($($arg:tt)*) => {{
        __mevi_dbg!($crate::LogType::Info, $($arg)*);
    }};
}

macro_rules! err {
    ($($arg:tt)*) => {{
        __mevi_dbg!($crate::LogType::Error, $($arg)*);
    }};
}
