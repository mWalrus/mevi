use std::fmt::Display;

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
    pub font_gc: Gcontext,
}

impl WindowState {
    fn new(
        window: Window,
        buffer: Pixmap,
        buffer_gc: Gcontext,
        image_pixmap: Pixmap,
        tile_gc: Gcontext,
        font_gc: Gcontext,
    ) -> Self {
        Self {
            window,
            buffer,
            buffer_gc,
            image_pixmap,
            tile_gc,
            font_gc,
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
    let image_pixmap = conn.generate_id()?;
    let buffer = conn.generate_id()?;
    let buffer_gc = conn.generate_id()?;
    let background_pixmap = conn.generate_id()?;
    let background_gc = conn.generate_id()?;
    let tile_gc = conn.generate_id()?;
    let font_gc = conn.generate_id()?;
    let font = conn.generate_id()?;

    let title = format!("{TITLE} - {file_path}");

    conn.open_font(
        font,
        "-misc-hack-medium-i-normal--0-0-0-0-m-0-ascii-0".as_bytes(),
    )?;

    conn.create_gc(
        font_gc,
        screen.root,
        &CreateGCAux::default()
            .font(font)
            .foreground(screen.black_pixel)
            .background(screen.white_pixel),
    )?;

    conn.close_font(font)?;

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
        font_gc,
    ))
}

pub struct DrawInfo {
    pub ix: i16,
    pub iy: i16,
    pub wx: i16,
    pub wy: i16,
    pub ww: u16,
    pub wh: u16,
    pub w: u16,
    pub h: u16,
}

impl Display for DrawInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "draw info: (ix: {}, iy: {}, wx: {}, wy: {}, w: {}, h: {})",
            self.ix, self.iy, self.wx, self.wy, self.w, self.h
        )
    }
}

pub fn calc_draw_info<C: Connection>(conn: &C, win: Window, iw: u16, ih: u16) -> Result<DrawInfo> {
    let attrs = conn.get_geometry(win)?.reply()?;
    let (ww, wh) = (attrs.width, attrs.height);
    let (cx, cy) = (ww as i16 / 2, wh as i16 / 2);

    let ix = cx - (iw as i16 / 2);
    let iy = cy - (ih as i16 / 2);

    let (ix, wx, w) = if iw > ww {
        (ix.abs(), 0, ww)
    } else {
        (0, ix, iw)
    };
    let (iy, wy, h) = if ih > wh {
        (iy.abs(), 0, wh)
    } else {
        (0, iy, ih)
    };

    let info = DrawInfo {
        ix,
        iy,
        wx,
        wy,
        ww,
        wh,
        w,
        h,
    };

    mevi_info!("{info}");

    Ok(info)
}
