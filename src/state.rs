use anyhow::Result;
use x11rb::{
    connection::Connection,
    protocol::{
        render::Picture,
        xproto::{Gcontext, Pixmap, Window},
    },
    rust_connection::RustConnection,
};

#[derive(Clone, Copy, Debug)]
pub struct MeviState {
    pub window: Window,
    pub menu: Window,
    pub pms: Pms,
    pub gcs: Gcs,
    pub pics: Pics,
}

#[derive(Clone, Copy, Debug)]
pub struct Gcs {
    pub buffer: Gcontext,
    pub background: Gcontext,
    pub tile: Gcontext,
}

#[derive(Clone, Copy, Debug)]
pub struct Pms {
    pub image: Pixmap,
    pub buffer: Pixmap,
    pub font_buffer: Pixmap,
    pub background: Pixmap,
}

#[derive(Clone, Copy, Debug)]
pub struct Pics {
    pub window: Picture,
    pub buffer: Picture,
    pub font_buffer: Picture,
}

impl MeviState {
    pub fn init(conn: &RustConnection) -> Result<Self> {
        let window = conn.generate_id()?;
        let menu = conn.generate_id()?;
        let pms = Pms {
            image: conn.generate_id()?,
            buffer: conn.generate_id()?,
            font_buffer: conn.generate_id()?,
            background: conn.generate_id()?,
        };
        let gcs = Gcs {
            buffer: conn.generate_id()?,
            background: conn.generate_id()?,
            tile: conn.generate_id()?,
        };
        let pics = Pics {
            window: conn.generate_id()?,
            buffer: conn.generate_id()?,
            font_buffer: conn.generate_id()?,
        };
        let state = Self {
            window,
            menu,
            pms,
            gcs,
            pics,
        };
        mevi_info!("Window: {window}");
        mevi_info!("Menu window: {menu}");
        mevi_info!("Pixmaps: {pms:?}");
        mevi_info!("Gcontexts: {gcs:?}");
        mevi_info!("Pictures: {pics:?}");
        Ok(state)
    }
}
