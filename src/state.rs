use anyhow::Result;
use x11rb::{
    connection::Connection,
    protocol::xproto::{Gcontext, Pixmap, Window},
    rust_connection::RustConnection,
};

#[derive(Clone, Copy, Debug)]
pub struct MeviState {
    pub window: Window,
    pub pms: Pms,
    pub gcs: Gcs,
}

#[derive(Clone, Copy, Debug)]
pub struct Gcs {
    pub buffer: Gcontext,
    pub background: Gcontext,
    pub tile: Gcontext,
    pub font: Gcontext,
    pub font_selected: Gcontext,
}

#[derive(Clone, Copy, Debug)]
pub struct Pms {
    pub image: Pixmap,
    pub buffer: Pixmap,
    pub background: Pixmap,
}

impl MeviState {
    pub fn init(conn: &RustConnection) -> Result<Self> {
        let window = conn.generate_id()?;
        let pms = Pms {
            image: conn.generate_id()?,
            buffer: conn.generate_id()?,
            background: conn.generate_id()?,
        };
        let gcs = Gcs {
            buffer: conn.generate_id()?,
            background: conn.generate_id()?,
            tile: conn.generate_id()?,
            font: conn.generate_id()?,
            font_selected: conn.generate_id()?,
        };
        Ok(Self { window, pms, gcs })
    }
}
