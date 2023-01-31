use anyhow::Result;
use x11rb::connection::Connection;
use x11rb::image::ColorComponent;
use x11rb::image::PixelLayout;
use x11rb::protocol::xproto::*;
use x11rb::protocol::Event;

mod img;
mod screen;
mod window;

pub static INITIAL_SIZE: (u16, u16) = (600, 800);
pub static TITLE: &'static str = "mevi";

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

    let (image, file_name) = img::get_image_from_args()?;
    let foreign_layout = PixelLayout::new(
        ColorComponent::new(8, 0)?,
        ColorComponent::new(8, 8)?,
        ColorComponent::new(8, 16)?,
    );
    let pixel_layout = screen::check_visual(screen, screen.root_visual);
    let image = image.reencode(foreign_layout, pixel_layout, conn.setup())?;

    let atoms = Atoms::new(conn)?.reply()?;
    let win_id = window::init_window(conn, screen, &atoms, &image, file_name)?;

    conn.map_window(win_id)?;
    conn.flush()?;

    loop {
        let event = conn.wait_for_event()?;

        match event {
            Event::Expose(evt) => if evt.count == 0 {},
            Event::ConfigureNotify(_) => {}
            Event::ClientMessage(evt) => {
                let data = evt.data.as_data32();
                if evt.format == 32 && evt.window == win_id && data[0] == atoms.WM_DELETE_WINDOW {
                    println!("Exit signal received");
                    break;
                }
            }
            Event::Error(e) => eprintln!("Received error: {e:?}"),
            _ => println!("Got an unknown event"),
        }
    }

    Ok(())
}
