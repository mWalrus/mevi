#[macro_use]
mod log;
#[macro_use]
mod util;
mod app;
mod cli;
mod event;
mod font;
mod img;
mod keys;
mod menu;
mod screen;
mod state;

use anyhow::Result;
use app::Mevi;
use clap::Parser;
use cli::Cli;
use img::MeviImage;
use lazy_static::lazy_static;
use log::LogType;
use x11rb::connection::Connection;

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
        _NET_WM_STATE,
        _NET_WM_STATE_FULLSCREEN,
    }
}

fn main() -> Result<()> {
    let (conn, screen_num) = x11rb::connect(None)?;
    info!("Connected to the X server");

    let screen = &conn.setup().roots[screen_num];
    info!("Got screen handle");

    let pixel_layout = screen::pixel_layout_from_visual(screen, screen.root_visual)?;

    let image = MeviImage::new(&conn, screen, &CLI.path, pixel_layout)?;

    let bg_img = img::get_bg_image(&conn, pixel_layout)?;

    let atoms = Atoms::new(&conn)?.reply()?;

    match Mevi::init(&conn, screen, atoms, image, bg_img) {
        Ok(mut mevi) => {
            info!("Initialized Mevi!");
            mevi.run_event_loop()?;
        }
        Err(e) => {
            err!("{e:?}");
            std::process::exit(1);
        }
    };

    Ok(())
}
