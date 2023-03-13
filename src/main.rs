mod cli;
mod font;
mod img;
mod keys;
#[macro_use]
mod log;
mod menu;
mod screen;
mod state;
#[macro_use]
mod util;
mod app;
mod event;

use anyhow::Result;
use app::Mevi;
use clap::Parser;
use cli::Cli;
use img::MeviImage;
use lazy_static::lazy_static;
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

    let pixel_layout = screen::pixel_layout_from_visual(screen, screen.root_visual)?;

    let image = MeviImage::new(&conn, screen, &CLI.path, pixel_layout)?;

    let bg_img = img::get_bg_image(&conn, pixel_layout)?;

    let atoms = Atoms::new(&conn)?.reply()?;

    match Mevi::init(&conn, screen, atoms, image, bg_img) {
        Ok(mut mevi) => mevi.run_event_loop()?,
        Err(e) => {
            mevi_err!("{e:?}");
            std::process::exit(1);
        }
    };

    Ok(())
}
