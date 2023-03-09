#[macro_export]
macro_rules! xy_in_rect {
    ($x:expr, $y:expr, $rect:expr) => {{
        let over_x = $x > $rect.x && $x < $rect.x + $rect.width as i16;
        let over_y = $y > $rect.y && $y < $rect.y + $rect.height as i16;
        over_x && over_y
    }};
}

#[macro_export]
macro_rules! key {
    ($code:expr) => {{
        use crate::keys::Key;
        match $code {
            9 => Key::Esc,
            31 => Key::I,
            36 => Key::Enter,
            58 => Key::M,
            111 => Key::Up,
            116 => Key::Down,
            _ => Key::Unknown,
        }
    }};
}
