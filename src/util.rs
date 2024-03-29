use crate::screen::RenderVisualInfo;
use anyhow::Result;
use std::fmt::Debug;
use x11rb::{
    connection::Connection,
    protocol::{
        render::{Color, ConnectionExt as _, CreatePictureAux, Picture, Repeat},
        xproto::{ConnectionExt, Rectangle, Window},
    },
};

pub static TITLE: &str = "mevi";
pub static INITIAL_SIZE: (u16, u16) = (600, 800);

pub static GRAY_RENDER_COLOR: Color = Color {
    red: 0x3b3b,
    green: 0x3b3b,
    blue: 0x3b3b,
    alpha: 0xffff,
};

pub static LIGHT_GRAY_RENDER_COLOR: Color = Color {
    red: 0x6666,
    green: 0x6666,
    blue: 0x6666,
    alpha: 0xffff,
};

pub static WHITE_RENDER_COLOR: Color = Color {
    red: 0xffff,
    green: 0xffff,
    blue: 0xffff,
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

#[derive(Debug, Clone, Copy)]
pub struct StatefulRenderPicture {
    pub active: RenderPicture,
    pub inactive: RenderPicture,
}

#[derive(Debug, Clone, Copy)]
pub struct RenderPicture {
    pub picture: Picture,
    pub fg: Color,
    pub bg: Color,
}

impl StatefulRenderPicture {
    pub fn new<C: Connection>(
        conn: &C,
        vis_info: &RenderVisualInfo,
        parent_id: Window,
        parent_w: u16,
        h: u16,
    ) -> Result<Self> {
        let active_pm = conn.generate_id()?;
        let active_pict = conn.generate_id()?;
        let inactive_pm = conn.generate_id()?;
        let inactive_pict = conn.generate_id()?;

        conn.create_pixmap(vis_info.root.depth, active_pm, parent_id, parent_w, h)?;

        conn.render_create_picture(
            active_pict,
            active_pm,
            vis_info.root.pict_format,
            &CreatePictureAux::default().repeat(Repeat::NORMAL),
        )?;

        conn.create_pixmap(vis_info.root.depth, inactive_pm, parent_id, parent_w, h)?;

        conn.render_create_picture(
            inactive_pict,
            inactive_pm,
            vis_info.root.pict_format,
            &CreatePictureAux::default().repeat(Repeat::NORMAL),
        )?;

        Ok(Self {
            active: RenderPicture {
                picture: active_pict,
                fg: WHITE_RENDER_COLOR,
                bg: LIGHT_GRAY_RENDER_COLOR,
            },
            inactive: RenderPicture {
                picture: inactive_pict,
                fg: WHITE_RENDER_COLOR,
                bg: GRAY_RENDER_COLOR,
            },
        })
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
