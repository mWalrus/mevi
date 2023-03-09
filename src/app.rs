use crate::event::MeviEvent;
use crate::img::MeviImage;
use crate::menu::{Menu, MenuAction};
use crate::util::{GRAY_COLOR, INITIAL_SIZE, TITLE};
use crate::{Atoms, CLI};
use anyhow::Result;
use std::fmt::Display;
use x11rb::connection::Connection;
use x11rb::image::Image;
use x11rb::protocol::xproto::{
    ConnectionExt, CreateGCAux, CreateWindowAux, EventMask, FillStyle, Gcontext, Pixmap, PropMode,
    Rectangle, Screen, Window, WindowClass,
};
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;

pub struct DrawInfo {
    pub child: Rectangle,
    pub parent: Rectangle,
}

impl Display for DrawInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "draw info: (parent: {:?}, child: {:?})",
            self.parent, self.child
        )
    }
}

pub struct Mevi<'a> {
    pub atoms: Atoms,
    conn: &'a RustConnection,
    pub window: Window,
    screen: &'a Screen,
    buffer: Pixmap,
    buffer_gc: Gcontext,
    image_pixmap: Pixmap,
    image: MeviImage,
    tile_gc: Gcontext,
    font_gc: Gcontext,
    needs_redraw: bool,
    pub menu: Menu,
    pub w: u16,
    pub h: u16,
    show_file_info: bool,
    should_exit: bool,
}

impl<'a> Mevi<'a> {
    pub fn init(
        conn: &'a RustConnection,
        screen: &'a Screen,
        atoms: Atoms,
        image: MeviImage,
        bg_img: Image,
    ) -> Result<Self> {
        let window = conn.generate_id()?;
        let image_pixmap = conn.generate_id()?;
        let buffer = conn.generate_id()?;
        let buffer_gc = conn.generate_id()?;
        let background_pixmap = conn.generate_id()?;
        let background_gc = conn.generate_id()?;
        let tile_gc = conn.generate_id()?;
        let font_gc = conn.generate_id()?;
        let font_gc_selected = conn.generate_id()?;
        let font = conn.generate_id()?;

        let path = CLI.path.to_string_lossy().to_string();
        let title = format!("{TITLE} - {path}");

        conn.open_font(font, "fixed".as_bytes())?;

        conn.create_gc(
            font_gc,
            screen.root,
            &CreateGCAux::default()
                .font(font)
                .foreground(screen.black_pixel)
                .background(screen.white_pixel),
        )?;

        conn.create_gc(
            font_gc_selected,
            screen.root,
            &CreateGCAux::default()
                .font(font)
                .foreground(screen.white_pixel)
                .background(GRAY_COLOR),
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
            image.w,
            image.h,
        )?;

        image.inner.put(conn, image_pixmap, buffer_gc, 0, 0)?;

        let win_aux = CreateWindowAux::default().event_mask(
            EventMask::EXPOSURE
                | EventMask::STRUCTURE_NOTIFY
                | EventMask::KEY_RELEASE
                | EventMask::BUTTON_PRESS
                | EventMask::POINTER_MOTION,
        );

        conn.create_window(
            screen.root_depth,
            window,
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
            window,
            atoms.WM_NAME,
            atoms.STRING,
            title.as_bytes(),
        )?;

        conn.change_property8(
            PropMode::REPLACE,
            window,
            atoms._NET_WM_NAME,
            atoms.UTF8_STRING,
            title.as_bytes(),
        )?;

        conn.change_property32(
            PropMode::REPLACE,
            window,
            atoms.WM_PROTOCOLS,
            atoms.ATOM,
            &[atoms.WM_DELETE_WINDOW],
        )?;

        conn.map_window(window)?;
        conn.flush()?;

        let menu = Menu::create(conn, screen, font_gc, font_gc_selected, window)?;

        Ok(Self {
            atoms,
            conn,
            window,
            screen,
            buffer,
            buffer_gc,
            image_pixmap,
            image,
            tile_gc,
            font_gc,
            needs_redraw: false,
            menu,
            w: INITIAL_SIZE.0,
            h: INITIAL_SIZE.1,
            show_file_info: CLI.info,
            should_exit: false,
        })
    }

    pub fn run_event_loop(&mut self) -> Result<()> {
        loop {
            let event = self.conn.wait_for_event()?;

            match MeviEvent::handle(&self, event) {
                MeviEvent::DrawImage => self.needs_redraw = true,
                MeviEvent::ToggleFileInfo => self.toggle_show_file_info(),
                MeviEvent::Menu(menu_evt) => match self.menu.handle_event(self.conn, menu_evt)? {
                    MenuAction::ToggleFileInfo => self.toggle_show_file_info(),
                    MenuAction::Exit => self.should_exit = true,
                    MenuAction::None => {}
                },
                MeviEvent::Exit => self.should_exit = true,
                MeviEvent::Error(e) => mevi_err!("{e:?}"),
                MeviEvent::Idle => {}
            }

            if self.should_exit {
                mevi_info!("Exit signal received");
                break;
            }

            if self.needs_redraw {
                self.draw_image()?;
            }
        }
        Ok(())
    }

    fn toggle_show_file_info(&mut self) {
        self.show_file_info = !self.show_file_info;
        self.needs_redraw = true;
    }

    fn draw_image(&mut self) -> Result<()> {
        let info = self.calc_image_draw_info()?;
        self.w = info.parent.width;
        self.h = info.parent.height;

        self.conn.create_pixmap(
            self.screen.root_depth,
            self.buffer,
            self.screen.root,
            info.parent.width,
            info.parent.height,
        )?;

        self.conn.poly_fill_rectangle(
            self.buffer,
            self.tile_gc,
            &[Rectangle {
                x: 0,
                y: 0,
                width: info.parent.width,
                height: info.parent.height,
            }],
        )?;

        self.conn.copy_area(
            self.image_pixmap,
            self.buffer,
            self.buffer_gc,
            info.child.x,
            info.child.y,
            info.parent.x,
            info.parent.y,
            info.child.width,
            info.child.height,
        )?;

        self.draw_file_info()?;

        self.conn.copy_area(
            self.buffer,
            self.window,
            self.buffer_gc,
            0,
            0,
            0,
            0,
            info.parent.width,
            info.parent.height,
        )?;

        self.conn.free_pixmap(self.buffer)?;
        self.conn.flush()?;
        self.needs_redraw = false;
        Ok(())
    }

    fn draw_file_info(&self) -> Result<()> {
        if self.show_file_info {
            self.conn.image_text8(
                self.buffer,
                self.font_gc,
                0,
                11,
                format!(
                    "path: {} | dimensions: {}x{} | type: {} | size: {}Kb",
                    self.image.path,
                    self.image.ow,
                    self.image.oh,
                    self.image.format,
                    self.image.size
                )
                .as_bytes(),
            )?;
        };
        Ok(())
    }

    fn calc_image_draw_info(&self) -> Result<DrawInfo> {
        let attrs = self.conn.get_geometry(self.window)?.reply()?;
        let (parent_w, parent_h) = (attrs.width, attrs.height);
        let (cx, cy) = (parent_w as i16 / 2, parent_h as i16 / 2);

        let child_x = cx - (self.image.w as i16 / 2);
        let child_y = cy - (self.image.h as i16 / 2);

        let (child_x, parent_x, child_w) = if self.image.w > parent_w {
            (child_x.abs(), 0, parent_w)
        } else {
            (0, child_x, self.image.w)
        };
        let (child_y, parent_y, child_h) = if self.image.h > parent_h {
            (child_y.abs(), 0, parent_h)
        } else {
            (0, child_y, self.image.h)
        };

        let info = DrawInfo {
            child: Rectangle {
                x: child_x,
                y: child_y,
                width: child_w,
                height: child_h,
            },
            parent: Rectangle {
                x: parent_x,
                y: parent_y,
                width: parent_w,
                height: parent_h,
            },
        };

        mevi_info!("{info}");

        Ok(info)
    }
}
