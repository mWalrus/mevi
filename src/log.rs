macro_rules! __mevi_dbg {
    ($type:expr, $($arg:tt)*) => {{
        if $crate::CLI.debug {
            use ::colored::Colorize;
            let t = match $type {
                $crate::LogType::Event => "[Event]".bold().blue(),
                $crate::LogType::Info => "[Info]".bold(),
            };
            println!("{}: {}", t, format_args!($($arg)*));
        }
    }};
}

macro_rules! mevi_event {
    ($arg:expr) => {{
        use x11rb::protocol::Event;
        match $arg {
            Event::MotionNotify(_) | Event::ConfigureNotify(_) | Event::Error(_) => {}
            _ => __mevi_dbg!($crate::LogType::Event, "{:?}", $arg),
        }
    }};
}

macro_rules! mevi_info {
    ($($arg:tt)*) => {{
        __mevi_dbg!($crate::LogType::Info, $($arg)*);
    }};
}

macro_rules! mevi_err {
    ($($arg:tt)*) => {{
        use ::colored::Colorize;
        println!("{} {}", "[Error]".bold().red(), format_args!($($arg)*));
    }};
}
