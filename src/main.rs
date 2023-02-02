use anyhow::Result;
use clap::Parser;
use cli::Cli;
use lazy_static::lazy_static;
use x11rb::connection::Connection;
use x11rb::image::ColorComponent;
use x11rb::image::PixelLayout;
use x11rb::protocol::xproto::*;
use x11rb::protocol::Event;

mod cli;
mod img;
#[macro_use]
mod log;
mod screen;
mod shm;
mod window;

pub static INITIAL_SIZE: (u16, u16) = (600, 800);
pub static TITLE: &'static str = "mevi";

pub(crate) enum LogType {
    Event,
    Info,
}

lazy_static! {
    static ref CLI: Cli = Cli::parse();
    static ref SHOULD_PRINT_DEBUG: bool = CLI.debug;
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
    let conn = &conn;

    match shm::check_shm_version(conn)? {
        None => {
            mevi_err!("X11 server does not support the SHM extension");
            return Ok(());
        }
        Some((major, minor)) => {
            if major < 1 || (major == 1 && minor < 2) {
                mevi_err!("X11 server supports version {major}.{minor} of the SHM extension, but version 1.2 is needed");
                return Ok(());
            }
        }
    }

    let screen = &conn.setup().roots[screen_num];

    let img = img::load_image(&CLI.path)?;
    let bg_img = img::get_bg_image()?;
    let foreign_layout = PixelLayout::new(
        ColorComponent::new(8, 0)?,
        ColorComponent::new(8, 8)?,
        ColorComponent::new(8, 16)?,
    );
    let pixel_layout = screen::check_visual(screen, screen.root_visual);

    let img = img.reencode(foreign_layout, pixel_layout, conn.setup())?;
    let img = img.native(conn.setup())?;

    let (iw, ih) = (img.width(), img.height());

    let bg_img = bg_img.reencode(foreign_layout, pixel_layout, conn.setup())?;

    let atoms = Atoms::new(conn)?.reply()?;

    let state = window::init_window(
        conn,
        screen,
        &atoms,
        &img,
        &bg_img,
        CLI.path.to_string_lossy().to_string(),
    )?;

    conn.map_window(state.win)?;
    conn.flush()?;

    loop {
        let event = conn.wait_for_event()?;

        match event {
            Event::Expose(e) => {
                mevi_event!(event);

                // FIXME: work out this shit
                conn.copy_area(
                    0,
                    state.win,
                    0,
                    e.x as _,
                    e.y as _,
                    e.x as _,
                    e.y as _,
                    e.width.min(iw),
                    e.height.min(ih),
                )?;
                if e.count == 0 {
                    conn.flush()?;
                }
            }
            Event::ClientMessage(evt) => {
                let data = evt.data.as_data32();
                if evt.format == 32 && evt.window == state.win && data[0] == atoms.WM_DELETE_WINDOW
                {
                    mevi_info!("Exit signal received");
                    break;
                }
            }
            Event::Error(e) => mevi_err!("Received error: {e:?}"),
            _ => {
                mevi_event!(event);
            } // ev => println!("Got an unknown event: {ev:?}"),
        }
    }

    Ok(())
}
