use anyhow::Result;
use x11rb::connection::Connection;
use x11rb::image::Image;
use x11rb::protocol::xproto::{
    BackingStore, ConnectionExt, CreateGCAux, CreateWindowAux, EventMask, Gcontext, Gravity,
    Pixmap, PropMode, Screen, Window, WindowClass,
};
use x11rb::wrapper::ConnectionExt as _;

use crate::{Atoms, INITIAL_SIZE, TITLE};

pub struct WindowState {
    pub win: Window,
    pub pm: Pixmap,
    pub gc: Gcontext,
}

impl WindowState {
    fn new(win: Window, pm: Pixmap, gc: Gcontext) -> Self {
        Self { win, pm, gc }
    }
}

pub struct Coordinate {
    pub x: i16,
    pub y: i16,
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

    let title = format!("{TITLE} - {file_path}");

    let (img_pm, img_gc) = create_pixmap_and_gc(conn, screen, img.width(), img.height())?;
    let (bg_img_pm, bg_img_gc) =
        create_pixmap_and_gc(conn, screen, bg_img.width(), bg_img.height())?;

    bg_img.put(conn, bg_img_pm, bg_img_gc, 0, 0)?;
    conn.free_gc(bg_img_gc)?;

    img.put(conn, img_pm, img_gc, 0, 0)?;

    let win_aux = CreateWindowAux::default()
        .event_mask(EventMask::EXPOSURE | EventMask::STRUCTURE_NOTIFY)
        .bit_gravity(Gravity::CENTER)
        .backing_store(BackingStore::NOT_USEFUL)
        .save_under(0)
        .override_redirect(0)
        .background_pixmap(bg_img_pm);

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

    conn.free_pixmap(bg_img_pm)?;

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

    Ok(WindowState::new(win_id, img_pm, img_gc))
}

fn create_pixmap_and_gc<'c, C: Connection>(
    conn: &'c C,
    s: &Screen,
    w: u16,
    h: u16,
) -> Result<(u32, u32)> {
    let pm = conn.generate_id()?;
    let gc = conn.generate_id()?;

    conn.create_gc(gc, s.root, &CreateGCAux::default().graphics_exposures(0))?;
    conn.create_pixmap(s.root_depth, pm, s.root, w, h)?;

    Ok((pm, gc))
}

pub fn center_coordinates<'c, C: Connection>(
    conn: &'c C,
    win: Window,
    iw: u16,
    ih: u16,
) -> Result<(i16, i16)> {
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
    Ok((ix, iy))
}
