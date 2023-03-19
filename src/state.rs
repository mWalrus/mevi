use std::fmt::Debug;

use anyhow::Result;
use x11rb::{
    connection::Connection,
    protocol::{
        render::PictureWrapper,
        xproto::{GcontextWrapper, PixmapWrapper, WindowWrapper},
    },
};

use crate::CLI;

pub struct MeviState<'s, C: Connection> {
    pub window: WindowWrapper<'s, C>,
    pub menu: WindowWrapper<'s, C>,
    pub pms: Pms<'s, C>,
    pub gcs: Gcs<'s, C>,
    pub pics: Pics<'s, C>,
    pub should_redraw: bool,
    pub should_exit: bool,
    pub draw_info: bool,
    pub fullscreen: bool,
}

pub struct Gcs<'s, C: Connection> {
    pub buffer: GcontextWrapper<'s, C>,
    pub background: GcontextWrapper<'s, C>,
    pub tile: GcontextWrapper<'s, C>,
}

pub struct Pms<'s, C: Connection> {
    pub image: PixmapWrapper<'s, C>,
    pub buffer: PixmapWrapper<'s, C>,
    pub font_buffer: PixmapWrapper<'s, C>,
    pub background: PixmapWrapper<'s, C>,
}

pub struct Pics<'s, C: Connection> {
    pub window: PictureWrapper<'s, C>,
    pub buffer: PictureWrapper<'s, C>,
    pub font_buffer: PictureWrapper<'s, C>,
}

impl<C: Connection> Debug for Gcs<'_, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Gcs {{ buffer: {}, background: {}, tile: {} }}",
            self.buffer.gcontext(),
            self.background.gcontext(),
            self.tile.gcontext()
        )
    }
}

impl<C: Connection> Debug for Pms<'_, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Pms {{ image: {}, buffer: {}, font_buffer: {}, background: {} }}",
            self.image.pixmap(),
            self.buffer.pixmap(),
            self.font_buffer.pixmap(),
            self.background.pixmap()
        )
    }
}

impl<C: Connection> Debug for Pics<'_, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Pics {{ window: {}, buffer: {}, font_buffer: {} }}",
            self.window.picture(),
            self.buffer.picture(),
            self.font_buffer.picture()
        )
    }
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

        mevi_info!("Window: {}", window.window());
        mevi_info!("Menu window: {}", menu.window());
        mevi_info!("Pixmaps: {pms:?}");
        mevi_info!("Gcontexts: {gcs:?}");
        mevi_info!("Pictures: {pics:?}");

        let state = Self {
            window,
            menu,
            pms,
            gcs,
            pics,
            should_redraw: false,
            should_exit: false,
            draw_info: CLI.info,
            fullscreen: false,
        };
        Ok(state)
    }
}
