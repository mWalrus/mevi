use anyhow::Result;
use lazy_static::lazy_static;
use x11rb::connection::Connection;
use x11rb::image::ColorComponent;
use x11rb::image::PixelLayout;
use x11rb::protocol::shm::{self, ConnectionExt as _};
use x11rb::protocol::xproto::{self, *};
use x11rb::protocol::Event;

mod cli;
mod img;
mod screen;
mod window;

pub static INITIAL_SIZE: (u16, u16) = (600, 800);
pub static TITLE: &'static str = "mevi";

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
                println!("EXPOSE: {e:?}");
                conn.copy_area(
                    state.pm,
                    state.win,
                    state.gc,
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
            Event::ConfigureNotify(_) => {}
            Event::ClientMessage(evt) => {
                let data = evt.data.as_data32();
                if evt.format == 32 && evt.window == state.win && data[0] == atoms.WM_DELETE_WINDOW
                {
                    println!("Exit signal received");
                    break;
                }
            }
            Event::Error(e) => eprintln!("Received error: {e:?}"),
            _ => {} // ev => println!("Got an unknown event: {ev:?}"),
        }
    }

    Ok(())
}
