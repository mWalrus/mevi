use crate::event::MeviEvent;
use crate::font::loader::LoadedFont;
use crate::font::{FontDrawer, RenderString, ToRenderLine};
use crate::img::MeviImage;
use crate::menu::{Menu, MenuAction};
use crate::screen::RenderVisualInfo;
use crate::state::MeviState;
use crate::util::{
    DrawInfo, GRAY_COLOR, GRAY_RENDER_COLOR, INITIAL_SIZE, TITLE, WHITE_RENDER_COLOR,
};
use crate::{Atoms, CLI};
use anyhow::Result;
use x11rb::connection::Connection;
use x11rb::image::Image;
use x11rb::protocol::render::{ConnectionExt as _, CreatePictureAux, PolyEdge, PolyMode, Repeat};
use x11rb::protocol::xproto::{
    ConnectionExt, CreateGCAux, CreateWindowAux, EventMask, FillStyle, PropMode, Rectangle, Screen,
    WindowClass,
};
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;

pub struct Mevi<'a> {
    pub atoms: Atoms,
    conn: &'a RustConnection,
    screen: &'a Screen,
    vis_info: RenderVisualInfo,
    file_info: RenderString,
    pub state: MeviState,
    pub font_drawer: FontDrawer,
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

        let vis_info = RenderVisualInfo::new(conn, screen)?;

        let menu = Menu::create(conn, screen, &state)?;
        let font = LoadedFont::new(conn, vis_info.render.pict_format)?;
        let font_drawer = FontDrawer::new(font);

        let image_info = image.to_lines(&font_drawer);
        let file_info = RenderString::new(image_info, GRAY_RENDER_COLOR, WHITE_RENDER_COLOR);
        conn.create_pixmap(
            screen.root_depth,
            state.pms.font_buffer,
            screen.root,
            file_info.total_width as u16,
            file_info.total_height,
        )?;

        conn.render_create_picture(
            state.pics.font_buffer,
            state.pms.font_buffer,
            vis_info.root.pict_format,
            &CreatePictureAux::default()
                .polyedge(PolyEdge::SMOOTH)
                .polymode(PolyMode::IMPRECISE),
        )?;

        Ok(Self {
            atoms,
            conn,
            screen,
            vis_info,
            file_info,
            state,
            font_drawer,
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
        let di = DrawInfo::calculate(self.conn, &self.state, &self.image)?;
        self.w = di.parent.w;
        self.h = di.parent.h;

        self.conn.create_pixmap(
            self.screen.root_depth,
            self.state.pms.buffer,
            self.screen.root,
            di.parent.w,
            di.parent.h,
        )?;

        self.conn.poly_fill_rectangle(
            self.state.pms.buffer,
            self.state.gcs.tile,
            &[Rectangle {
                x: 0,
                y: 0,
                width: di.parent.w,
                height: di.parent.h,
            }],
        )?;

        self.conn.copy_area(
            self.state.pms.image,
            self.state.pms.buffer,
            self.state.gcs.buffer,
            di.child.x,
            di.child.y,
            di.parent.x,
            di.parent.y,
            di.child.w,
            di.child.h,
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
            di.parent.w,
            di.parent.h,
        )?;

        self.conn.free_pixmap(self.state.pms.buffer)?;
        self.conn.flush()?;
        self.needs_redraw = false;
        Ok(())
    }

    fn draw_file_info(&self) -> Result<()> {
        if self.show_file_info {
            self.conn.render_create_picture(
                self.state.pics.buffer,
                self.state.pms.buffer,
                self.vis_info.root.pict_format,
                &CreatePictureAux::default().repeat(Repeat::NORMAL),
            )?;

            self.font_drawer
                .draw(self.conn, &self.state, &self.file_info, 5, 0)?;

            self.conn.render_free_picture(self.state.pics.buffer)?;
        }
        Ok(())
    }
}
