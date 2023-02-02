macro_rules! __mevi_dbg {
    ($type:expr, $($arg:tt)*) => {{
        if *$crate::SHOULD_PRINT_DEBUG {
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
    ($arg:tt) => {{
        __mevi_dbg!($crate::LogType::Event, "{:?}", $arg);
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
        println!("{}: {}", "[Error]".bold().red(), format_args!($($arg)*));
    }};
}
