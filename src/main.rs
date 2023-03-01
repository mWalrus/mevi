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

    let screen = &conn.setup().roots[screen_num];

    let image_wrapper = img::load_image(
        &CLI.path,
        screen.width_in_pixels as u32,
        screen.height_in_pixels as u32,
    )?;
    let bg_img = img::get_bg_image()?;

    let foreign_layout = PixelLayout::new(
        ColorComponent::new(8, 0)?,
        ColorComponent::new(8, 8)?,
        ColorComponent::new(8, 16)?,
    );

    let pixel_layout = screen::check_visual(screen, screen.root_visual);

    let img = image_wrapper
        .image
        .reencode(foreign_layout, pixel_layout, conn.setup())?;

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

    loop {
        let event = conn.wait_for_event()?;

        match event {
            Event::Expose(e) if e.count == 0 => {
                mevi_event!(event);

                let info = window::calc_draw_info(
                    conn,
                    state.window,
                    image_wrapper.width,
                    image_wrapper.height,
                )?;

                conn.create_pixmap(
                    screen.root_depth,
                    state.buffer,
                    screen.root,
                    info.ww,
                    info.wh,
                )?;

                conn.poly_fill_rectangle(
                    state.buffer,
                    state.tile_gc,
                    &[Rectangle {
                        x: 0,
                        y: 0,
                        width: info.ww,
                        height: info.wh,
                    }],
                )?;

                conn.copy_area(
                    state.image_pixmap,
                    state.buffer,
                    state.buffer_gc,
                    info.ix,
                    info.iy,
                    info.wx,
                    info.wy,
                    info.w,
                    info.h,
                )?;

                conn.image_text8(
                    state.buffer,
                    state.font_gc,
                    0,
                    15,
                    image_wrapper.path.as_bytes(),
                )?;

                conn.copy_area(
                    state.buffer,
                    state.window,
                    state.buffer_gc,
                    0,
                    0,
                    0,
                    0,
                    info.ww,
                    info.wh,
                )?;

                conn.free_pixmap(state.buffer)?;
                conn.flush()?;
            }
            Event::ClientMessage(evt) => {
                let data = evt.data.as_data32();
                if evt.format == 32
                    && evt.window == state.window
                    && data[0] == atoms.WM_DELETE_WINDOW
                {
                    mevi_info!("Exit signal received");
                    break;
                }
            }
            Event::Error(e) => mevi_err!("Received error: {e:?}"),
            _ => {}
        }
    }

    Ok(())
}
