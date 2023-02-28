use anyhow::Result;
use x11rb::connection::Connection;
use x11rb::image::Image;
use x11rb::protocol::xproto::{
    ConnectionExt, CreateGCAux, CreateWindowAux, EventMask, FillStyle, Gcontext, Pixmap, PropMode,
    Screen, Window, WindowClass,
};
use x11rb::wrapper::ConnectionExt as _;

use crate::{Atoms, INITIAL_SIZE, TITLE};

pub struct WindowState {
    pub window: Window,
    pub buffer: Pixmap,
    pub buffer_gc: Gcontext,
    pub image_pixmap: Pixmap,
    pub tile_gc: Gcontext,
}

impl WindowState {
    fn new(
        window: Window,
        buffer: Pixmap,
        buffer_gc: Gcontext,
        image_pixmap: Pixmap,
        tile_gc: Gcontext,
    ) -> Self {
        Self {
            window,
            buffer,
            buffer_gc,
            image_pixmap,
            tile_gc,
        }
    }
}

pub fn init_window(
    conn: &impl Connection,
    screen: &Screen,
    atoms: &Atoms,
    img: &Image,
    bg_img: &Image,
    file_path: String,
) -> Result<WindowState> {
    let win_id = conn.generate_id()?;
    let buffer = conn.generate_id()?;
    let buffer_gc = conn.generate_id()?;
    let background_pixmap = conn.generate_id()?;
    let image_pixmap = conn.generate_id()?;
    let background_gc = conn.generate_id()?;
    let tile_gc = conn.generate_id()?;

    let title = format!("{TITLE} - {file_path}");

    conn.create_pixmap(
        screen.root_depth,
        background_pixmap,
        screen.root,
        bg_img.width(),
        bg_img.height(),
    )?;

    conn.create_gc(
        background_gc,
        screen.root,
        &CreateGCAux::default().graphics_exposures(0),
    )?;

    bg_img.put(conn, background_pixmap, background_gc, 0, 0)?;

    conn.create_gc(
        tile_gc,
        screen.root,
        &CreateGCAux::default()
            .fill_style(Some(FillStyle::TILED))
            .tile(background_pixmap),
    )?;

    conn.free_gc(background_gc)?;
    conn.free_pixmap(background_pixmap)?;

    conn.create_gc(
        buffer_gc,
        screen.root,
        &CreateGCAux::default().graphics_exposures(0),
    )?;

    conn.create_pixmap(
        screen.root_depth,
        image_pixmap,
        screen.root,
        img.width(),
        img.height(),
    )?;

    img.put(conn, image_pixmap, buffer_gc, 0, 0)?;

    let win_aux =
        CreateWindowAux::default().event_mask(EventMask::EXPOSURE | EventMask::STRUCTURE_NOTIFY);

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
        &win_aux,
    )?;

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

    conn.map_window(win_id)?;
    conn.flush()?;

    Ok(WindowState::new(
        win_id,
        buffer,
        buffer_gc,
        image_pixmap,
        tile_gc,
    ))
}

pub fn center_coordinates<C: Connection>(
    conn: &C,
    win: Window,
    iw: u16,
    ih: u16,
) -> Result<(i16, i16, u16, u16)> {
    let attrs = conn.get_geometry(win)?.reply()?;
    mevi_info!("Image dimensions: {iw}x{ih}");
    let (cx, cy) = (attrs.width as i16 / 2, attrs.height as i16 / 2);
    mevi_info!(
        "Center of window with size {}x{}: x -> {cx}, y -> {cy}",
        attrs.width,
        attrs.height
    );
    let ix = cx - (iw as i16 / 2);
    let iy = cy - (ih as i16 / 2);
    mevi_info!("Position to start drawing from: x -> {ix}, y -> {iy}");
    Ok((ix, iy, attrs.width, attrs.height))
}
