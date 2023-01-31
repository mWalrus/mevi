use anyhow::Result;
use x11rb::connection::Connection;
use x11rb::image::Image;
use x11rb::protocol::xproto::{
    ConnectionExt, CreateGCAux, CreateWindowAux, PropMode, Screen, Window, WindowClass,
};
use x11rb::wrapper::ConnectionExt as _;

use crate::{Atoms, INITIAL_SIZE, TITLE};

pub fn init_window(
    conn: &impl Connection,
    screen: &Screen,
    atoms: &Atoms,
    image: &Image,
    file_name: String,
) -> Result<Window> {
    let win_id = conn.generate_id()?;
    let pixmap_id = conn.generate_id()?;
    let gc_id = conn.generate_id()?;

    let title = format!("{TITLE} - {file_name}");

    conn.create_gc(
        gc_id,
        screen.root,
        &CreateGCAux::default().graphics_exposures(0),
    )?;

    conn.create_pixmap(
        screen.root_depth,
        pixmap_id,
        screen.root,
        image.width(),
        image.height(),
    )?;

    image.put(conn, pixmap_id, gc_id, 0, 0)?;

    conn.free_gc(gc_id)?;

    conn.create_window(
        screen.root_depth,
        win_id,
        screen.root,
        0,
        0,
        INITIAL_SIZE.0,
        INITIAL_SIZE.1,
        0,
        WindowClass::INPUT_OUTPUT,
        0,
        &CreateWindowAux::default().background_pixmap(pixmap_id),
    )?;

    conn.free_pixmap(pixmap_id)?;

    conn.change_property8(
        PropMode::REPLACE,
        win_id,
        atoms.WM_NAME,
        atoms.STRING,
        title.as_bytes(),
    )?;

    conn.change_property8(
        PropMode::REPLACE,
        win_id,
        atoms._NET_WM_NAME,
        atoms.UTF8_STRING,
        title.as_bytes(),
    )?;

    conn.change_property32(
        PropMode::REPLACE,
        win_id,
        atoms.WM_PROTOCOLS,
        atoms.ATOM,
        &[atoms.WM_DELETE_WINDOW],
    )?;

    Ok(win_id)
}
