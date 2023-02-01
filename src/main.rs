use anyhow::Result;
use x11rb::connection::Connection;
use x11rb::image::ColorComponent;
use x11rb::image::PixelLayout;
use x11rb::protocol::xproto::{self, *};
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

    let (image, file_path) = img::get_image_from_args()?;
    let bg_image = img::get_bg_image()?;
    let foreign_layout = PixelLayout::new(
        ColorComponent::new(8, 0)?,
        ColorComponent::new(8, 8)?,
        ColorComponent::new(8, 16)?,
    );
    let pixel_layout = screen::check_visual(screen, screen.root_visual);

    let image = image.reencode(foreign_layout, pixel_layout, conn.setup())?;
    let (img_width, img_height) = (image.width(), image.height());

    let bg_image = bg_image.reencode(foreign_layout, pixel_layout, conn.setup())?;

    let atoms = Atoms::new(conn)?.reply()?;
    let (win_id, pixmap_id, gc_id) =
        window::init_window(conn, screen, &atoms, &image, &bg_image, file_path)?;

    conn.map_window(win_id)?;
    conn.flush()?;

    let mut is_first_iteration = true;

    loop {
        let event = conn.wait_for_event()?;

        match event {
            Event::Expose(evt) => {
                println!("EXPOSE: {evt:?}");
                if is_first_iteration {
                    is_first_iteration = false;
                    xproto::copy_area(
                        conn, pixmap_id, win_id, gc_id, 0, 0, 0, 0, img_width, img_height,
                    )?;
                } else {
                    xproto::copy_area(
                        conn,
                        pixmap_id,
                        win_id,
                        gc_id,
                        evt.x as i16,
                        evt.y as i16,
                        evt.x as i16,
                        evt.y as i16,
                        evt.width,
                        evt.height,
                    )?;
                };
                conn.flush()?;
            }
            Event::ConfigureNotify(_) => {}
            Event::ClientMessage(evt) => {
                let data = evt.data.as_data32();
                if evt.format == 32 && evt.window == win_id && data[0] == atoms.WM_DELETE_WINDOW {
                    println!("Exit signal received");
                    break;
                }
            }
            Event::Error(e) => eprintln!("Received error: {e:?}"),
            ev => println!("Got an unknown event: {ev:?}"),
        }
        conn.flush()?;
    }

    Ok(())
}
