use std::fmt::Debug;

use anyhow::Result;
use x11rb::{
    connection::Connection,
    protocol::{
        render::PictureWrapper,
        xproto::{GcontextWrapper, PixmapWrapper, WindowWrapper},
    },
};

#[derive(Debug)]
pub struct MeviState<'s, C: Connection> {
    pub window: WindowWrapper<'s, C>,
    pub menu: WindowWrapper<'s, C>,
    pub pms: Pms<'s, C>,
    pub gcs: Gcs<'s, C>,
    pub pics: Pics<'s, C>,
}

#[derive(Debug)]
pub struct Gcs<'s, C: Connection> {
    pub buffer: GcontextWrapper<'s, C>,
    pub background: GcontextWrapper<'s, C>,
    pub tile: GcontextWrapper<'s, C>,
}

#[derive(Debug)]
pub struct Pms<'s, C: Connection> {
    pub image: PixmapWrapper<'s, C>,
    pub buffer: PixmapWrapper<'s, C>,
    pub font_buffer: PixmapWrapper<'s, C>,
    pub background: PixmapWrapper<'s, C>,
}

#[derive(Debug)]
pub struct Pics<'s, C: Connection> {
    pub window: PictureWrapper<'s, C>,
    pub buffer: PictureWrapper<'s, C>,
    pub font_buffer: PictureWrapper<'s, C>,
}

impl<'s, C: Connection + Debug> MeviState<'s, C> {
    pub fn init(conn: &'s C) -> Result<Self> {
        let window = WindowWrapper::for_window(conn, conn.generate_id()?);
        let menu = WindowWrapper::for_window(conn, conn.generate_id()?);
        let pms = Pms {
            image: PixmapWrapper::for_pixmap(conn, conn.generate_id()?),
            buffer: PixmapWrapper::for_pixmap(conn, conn.generate_id()?),
            font_buffer: PixmapWrapper::for_pixmap(conn, conn.generate_id()?),
            background: PixmapWrapper::for_pixmap(conn, conn.generate_id()?),
        };
        let gcs = Gcs {
            buffer: GcontextWrapper::for_gcontext(conn, conn.generate_id()?),
            background: GcontextWrapper::for_gcontext(conn, conn.generate_id()?),
            tile: GcontextWrapper::for_gcontext(conn, conn.generate_id()?),
        };
        let pics = Pics {
            window: PictureWrapper::for_picture(conn, conn.generate_id()?),
            buffer: PictureWrapper::for_picture(conn, conn.generate_id()?),
            font_buffer: PictureWrapper::for_picture(conn, conn.generate_id()?),
        };

        mevi_info!("Window: {window:?}");
        mevi_info!("Menu window: {menu:?}");
        mevi_info!("Pixmaps: {pms:?}");
        mevi_info!("Gcontexts: {gcs:?}");
        mevi_info!("Pictures: {pics:?}");

        let state = Self {
            window,
            menu,
            pms,
            gcs,
            pics,
        };
        Ok(state)
    }
}
