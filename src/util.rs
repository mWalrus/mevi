use std::fmt::Display;

use anyhow::Result;
use x11rb::{
    protocol::{
        render::Color,
        xproto::{ConnectionExt, Rectangle},
    },
    rust_connection::RustConnection,
};

use crate::{img::MeviImage, state::MeviState};

pub static TITLE: &str = "mevi";
pub static GRAY_COLOR: u32 = 0x3b3b3b;
pub static INITIAL_SIZE: (u16, u16) = (600, 800);
pub static MENU_ITEM_HEIGHT: u16 = 20;

pub static GRAY_RENDER_COLOR: Color = Color {
    red: 0x3b3b,
    green: 0x3b3b,
    blue: 0x3b3b,
    alpha: 0xffff,
};

pub static WHITE_RENDER_COLOR: Color = Color {
    red: 0xffff,
    green: 0xffff,
    blue: 0xffff,
    alpha: 0xffff,
};

pub static BLACK_RENDER_COLOR: Color = Color {
    red: 0x0000,
    green: 0x0000,
    blue: 0x0000,
    alpha: 0xffff,
};

#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x: i16,
    pub y: i16,
    pub w: u16,
    pub h: u16,
}

impl Rect {
    pub fn new(x: i16, y: i16, w: u16, h: u16) -> Self {
        Self { x, y, w, h }
    }
}

impl From<Rect> for Rectangle {
    fn from(r: Rect) -> Self {
        Self {
            x: r.x,
            y: r.y,
            width: r.w,
            height: r.h,
        }
    }
}

pub struct DrawInfo {
    pub child: Rect,
    pub parent: Rect,
}

impl DrawInfo {
    pub fn calculate(conn: &RustConnection, state: &MeviState, image: &MeviImage) -> Result<Self> {
        let attrs = conn.get_geometry(state.window)?.reply()?;
        let (parent_w, parent_h) = (attrs.width, attrs.height);
        let (cx, cy) = (parent_w as i16 / 2, parent_h as i16 / 2);

        let child_x = cx - (image.w as i16 / 2);
        let child_y = cy - (image.h as i16 / 2);

        let (child_x, parent_x, child_w) = if image.w > parent_w {
            (child_x.abs(), 0, parent_w)
        } else {
            (0, child_x, image.w)
        };
        let (child_y, parent_y, child_h) = if image.h > parent_h {
            (child_y.abs(), 0, parent_h)
        } else {
            (0, child_y, image.h)
        };

        let info = DrawInfo {
            child: Rect::new(child_x, child_y, child_w, child_h),
            parent: Rect::new(parent_x, parent_y, parent_w, parent_h),
        };

        mevi_info!("{info}");

        Ok(info)
    }
}

impl Display for DrawInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "draw info: (parent: {:?}, child: {:?})",
            self.parent, self.child
        )
    }
}

#[macro_export]
macro_rules! xy_in_rect {
    ($x:expr, $y:expr, $rect:expr) => {{
        let over_x = $x > $rect.x && $x < $rect.x + $rect.width as i16;
        let over_y = $y > $rect.y && $y < $rect.y + $rect.height as i16;
        over_x && over_y
    }};
}
