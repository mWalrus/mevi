#[derive(PartialEq, PartialOrd)]
pub(crate) enum LogType {
    Event,
    Info,
    Error,
}

macro_rules! __mevi_dbg {
    ($type:expr, $($arg:tt)*) => {{
        use ::chrono::Local;
        use ::colored::Colorize;

        let now = Local::now();
        let type_ = match $type {
            $crate::LogType::Event => "[Event]".bold().blue(),
            $crate::LogType::Info => "[Info]".bold().green(),
            $crate::LogType::Error => "[Error]".bold().red(),
        };
        println!("{now:?} {type_} {}", format_args!($($arg)*))
    }};
}

macro_rules! event {
    ($arg:expr) => {{
        use x11rb::protocol::Event;
        use $crate::CLI;
        match $arg {
            Event::MotionNotify(_) | Event::ConfigureNotify(_) | Event::Error(_) => {}
            _ if CLI.debug => __mevi_dbg!($crate::LogType::Event, "{:?}", $arg),
            _ => {}
        }
    }};
}

macro_rules! info {
    ($($arg:tt)*) => {{
        use $crate::CLI;
        if CLI.debug {
            __mevi_dbg!($crate::LogType::Info, $($arg)*);
        }
    }};
}

macro_rules! err {
    ($($arg:tt)*) => {{
        __mevi_dbg!($crate::LogType::Error, $($arg)*);
    }};
}
