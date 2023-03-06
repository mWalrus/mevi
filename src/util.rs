#[macro_export]
macro_rules! xy_in_rect {
    ($x:expr, $y:expr, $rect:expr) => {{
        let over_x = $x > $rect.x && $x < $rect.x + $rect.width as i16;
        let over_y = $y > $rect.y && $y < $rect.y + $rect.height as i16;
        over_x && over_y
    }};
}
