pub enum Key {
    F,
    I,
    M,
    Up,
    Down,
    Esc,
    Enter,
    Unknown,
}

impl From<u8> for Key {
    fn from(value: u8) -> Self {
        match value {
            9 => Key::Esc,
            31 => Key::I,
            36 => Key::Enter,
            41 => Key::F,
            58 => Key::M,
            111 => Key::Up,
            116 => Key::Down,
            _ => Key::Unknown,
        }
    }
}
