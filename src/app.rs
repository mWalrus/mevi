use crate::event::MeviEvent;
use crate::img::MeviImage;
use crate::menu::{Menu, MenuAction};
use crate::state::MeviState;
use crate::util::{GRAY_COLOR, INITIAL_SIZE, TITLE};
use crate::{Atoms, CLI};
use anyhow::Result;
use std::fmt::Display;
use x11rb::connection::Connection;
use x11rb::image::Image;
use x11rb::protocol::xproto::{
    ConnectionExt, CreateGCAux, CreateWindowAux, EventMask, FillStyle, PropMode, Rectangle, Screen,
    WindowClass,
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
    screen: &'a Screen,
    pub state: MeviState,
    image: MeviImage,
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
        let state = MeviState::init(conn)?;

        let path = CLI.path.to_string_lossy().to_string();
        let title = format!("{TITLE} - {path}");

        let font = conn.generate_id()?;
        conn.open_font(font, "fixed".as_bytes())?;

        conn.create_gc(
            state.gcs.font,
            screen.root,
            &CreateGCAux::default()
                .font(font)
                .foreground(screen.black_pixel)
                .background(screen.white_pixel),
        )?;

        conn.create_gc(
            state.gcs.font_selected,
            screen.root,
            &CreateGCAux::default()
                .font(font)
                .foreground(screen.white_pixel)
                .background(GRAY_COLOR),
        )?;

        conn.close_font(font)?;

        conn.create_pixmap(
            screen.root_depth,
            state.pms.background,
            screen.root,
            bg_img.width(),
            bg_img.height(),
        )?;

        conn.create_gc(
            state.gcs.background,
            screen.root,
            &CreateGCAux::default().graphics_exposures(0),
        )?;

        bg_img.put(conn, state.pms.background, state.gcs.background, 0, 0)?;

        conn.create_gc(
            state.gcs.tile,
            screen.root,
            &CreateGCAux::default()
                .fill_style(Some(FillStyle::TILED))
                .tile(state.pms.background),
        )?;

        conn.free_gc(state.gcs.background)?;
        conn.free_pixmap(state.pms.background)?;

        conn.create_gc(
            state.gcs.buffer,
            screen.root,
            &CreateGCAux::default().graphics_exposures(0),
        )?;

        conn.create_pixmap(
            screen.root_depth,
            state.pms.image,
            screen.root,
            image.w,
            image.h,
        )?;

        image
            .inner
            .put(conn, state.pms.image, state.gcs.buffer, 0, 0)?;

        let win_aux = CreateWindowAux::default().event_mask(
            EventMask::EXPOSURE
                | EventMask::STRUCTURE_NOTIFY
                | EventMask::KEY_RELEASE
                | EventMask::BUTTON_PRESS
                | EventMask::POINTER_MOTION,
        );

        conn.create_window(
            screen.root_depth,
            state.window,
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
            state.window,
            atoms.WM_NAME,
            atoms.STRING,
            title.as_bytes(),
        )?;

        conn.change_property8(
            PropMode::REPLACE,
            state.window,
            atoms._NET_WM_NAME,
            atoms.UTF8_STRING,
            title.as_bytes(),
        )?;

        conn.change_property32(
            PropMode::REPLACE,
            state.window,
            atoms.WM_PROTOCOLS,
            atoms.ATOM,
            &[atoms.WM_DELETE_WINDOW],
        )?;

        conn.map_window(state.window)?;
        conn.flush()?;

        let menu = Menu::create(
            conn,
            screen,
            state.gcs.font,
            state.gcs.font_selected,
            state.window,
        )?;

        Ok(Self {
            atoms,
            conn,
            screen,
            state,
            image,
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

            match MeviEvent::handle(self, event) {
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
            self.state.pms.buffer,
            self.screen.root,
            info.parent.width,
            info.parent.height,
        )?;

        self.conn.poly_fill_rectangle(
            self.state.pms.buffer,
            self.state.gcs.tile,
            &[Rectangle {
                x: 0,
                y: 0,
                width: info.parent.width,
                height: info.parent.height,
            }],
        )?;

        self.conn.copy_area(
            self.state.pms.image,
            self.state.pms.buffer,
            self.state.gcs.buffer,
            info.child.x,
            info.child.y,
            info.parent.x,
            info.parent.y,
            info.child.width,
            info.child.height,
        )?;

        self.draw_file_info()?;

        self.conn.copy_area(
            self.state.pms.buffer,
            self.state.window,
            self.state.gcs.buffer,
            0,
            0,
            0,
            0,
            info.parent.width,
            info.parent.height,
        )?;

        self.conn.free_pixmap(self.state.pms.buffer)?;
        self.conn.flush()?;
        self.needs_redraw = false;
        Ok(())
    }

    fn draw_file_info(&self) -> Result<()> {
        if self.show_file_info {
            self.conn.image_text8(
                self.state.pms.buffer,
                self.state.gcs.font,
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
        let attrs = self.conn.get_geometry(self.state.window)?.reply()?;
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
