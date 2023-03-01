mod cli;
mod img;
#[macro_use]
mod log;
mod screen;
mod window;

use anyhow::Result;
use clap::Parser;
use cli::Cli;
use lazy_static::lazy_static;
use window::Mevi;
use x11rb::connection::Connection;

pub(crate) enum LogType {
    Event,
    Info,
}

lazy_static! {
    static ref CLI: Cli = Cli::parse();
}

x11rb::atom_manager! {
    pub Atoms: AtomsCookie {
        WM_PROTOCOLS,
        WM_DELETE_WINDOW,
        UTF8_STRING,
        ATOM,
        WM_NAME,
        STRING,
        _NET_WM_NAME,
    }
}

fn main() -> Result<()> {
    let (conn, screen_num) = x11rb::connect(None)?;

    let screen = &conn.setup().roots[screen_num];

    let (image, orig_w, orig_h) = img::load_image(
        &CLI.path,
        screen.width_in_pixels as u32,
        screen.height_in_pixels as u32,
    )?;
    let bg_img = img::get_bg_image()?;

    let atoms = Atoms::new(&conn)?.reply()?;

    let mevi = Mevi::init(&conn, screen, atoms, &image, orig_w, orig_h, &bg_img)?;

    mevi.run()?;

    Ok(())
}
