use std::fmt::Debug;
use std::rc::Rc;

use crate::event::MeviEvent;
use crate::font::loader::LoadedFont;
use crate::font::{FontDrawer, RenderString, ToRenderLine};
use crate::img::MeviImage;
use crate::menu::{Menu, MenuAction};
use crate::screen::RenderVisualInfo;
use crate::state::MeviState;
use crate::util::{DrawInfo, Rect, GRAY_RENDER_COLOR, INITIAL_SIZE, TITLE};
use crate::{Atoms, CLI};
use anyhow::Result;
use x11rb::connection::Connection;
use x11rb::image::Image;
use x11rb::protocol::render::{ConnectionExt as _, CreatePictureAux, PolyEdge, PolyMode, Repeat};
use x11rb::protocol::xproto::{
    ConnectionExt, CreateGCAux, CreateWindowAux, EventMask, FillStyle, PropMode, Screen,
    WindowClass,
};
use x11rb::wrapper::ConnectionExt as _;

pub struct Mevi<'a, C: Connection> {
    pub atoms: Atoms,
    conn: Rc<&'a C>,
    screen: &'a Screen,
    vis_info: Rc<RenderVisualInfo>,
    file_info: RenderString,
    pub state: MeviState<'a, C>,
    pub font_drawer: Rc<FontDrawer>,
    image: MeviImage,
    needs_redraw: bool,
    pub menu: Menu<'a, C>,
    pub w: u16,
    pub h: u16,
    show_file_info: bool,
    should_exit: bool,
}

impl<'a, C: Connection + Debug> Mevi<'a, C> {
    pub fn init(
        conn: &'a C,
        screen: &'a Screen,
        atoms: Atoms,
        image: MeviImage,
        bg_img: Image,
    ) -> Result<Self> {
        let state = MeviState::init(conn)?;
        let vis_info = Rc::new(RenderVisualInfo::new(conn, screen)?);
        let font = LoadedFont::new(conn, vis_info.render.pict_format)?;
        let font_drawer = Rc::new(FontDrawer::new(font));

        let path = CLI.path.to_string_lossy().to_string();
        let title = format!("{TITLE} - {path}");
        let image_info = image.to_lines(&font_drawer);
        let file_info = RenderString::new(image_info).line_gap(5).pad(5);

        Self::set_bg(conn, &state, screen, bg_img)?;
        Self::set_image(conn, &state, screen, &image)?;
        Self::init_file_info_font_buffer(conn, &state, screen, &vis_info, &file_info)?;

        let win_aux = CreateWindowAux::default().event_mask(
            EventMask::EXPOSURE
                | EventMask::STRUCTURE_NOTIFY
                | EventMask::KEY_RELEASE
                | EventMask::BUTTON_PRESS
                | EventMask::POINTER_MOTION,
        );

        conn.create_window(
            screen.root_depth,
            state.window.window(),
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

        mevi_info!("Created main window");

        conn.change_property8(
            PropMode::REPLACE,
            state.window.window(),
            atoms.WM_NAME,
            atoms.STRING,
            title.as_bytes(),
        )?;

        conn.change_property8(
            PropMode::REPLACE,
            state.window.window(),
            atoms._NET_WM_NAME,
            atoms.UTF8_STRING,
            title.as_bytes(),
        )?;

        conn.change_property32(
            PropMode::REPLACE,
            state.window.window(),
            atoms.WM_PROTOCOLS,
            atoms.ATOM,
            &[atoms.WM_DELETE_WINDOW],
        )?;
        mevi_info!("Set main window properties");

        conn.map_window(state.window.window())?;
        conn.flush()?;
        mevi_info!("Mapped the main window");

        let conn = Rc::new(conn);
        let menu = Menu::create(
            Rc::clone(&conn),
            screen,
            state.window.window(),
            Rc::clone(&vis_info),
            Rc::clone(&font_drawer),
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

    pub fn set_bg(conn: &C, st: &MeviState<C>, sc: &Screen, i: Image) -> Result<()> {
        conn.create_pixmap(
            sc.root_depth,
            st.pms.background.pixmap(),
            sc.root,
            i.width(),
            i.height(),
        )?;

        conn.create_gc(
            st.gcs.background.gcontext(),
            sc.root,
            &CreateGCAux::default().graphics_exposures(0),
        )?;

        i.put(
            conn,
            st.pms.background.pixmap(),
            st.gcs.background.gcontext(),
            0,
            0,
        )?;

        conn.create_gc(
            st.gcs.tile.gcontext(),
            sc.root,
            &CreateGCAux::default()
                .fill_style(Some(FillStyle::TILED))
                .tile(st.pms.background.pixmap()),
        )?;

        conn.free_gc(st.gcs.background.gcontext())?;
        conn.free_pixmap(st.pms.background.pixmap())?;
        Ok(())
    }

    pub fn set_image(conn: &C, st: &MeviState<C>, sc: &Screen, i: &MeviImage) -> Result<()> {
        conn.create_gc(
            st.gcs.buffer.gcontext(),
            sc.root,
            &CreateGCAux::default().graphics_exposures(0),
        )?;

        conn.create_pixmap(sc.root_depth, st.pms.image.pixmap(), sc.root, i.w, i.h)?;

        i.inner
            .put(conn, st.pms.image.pixmap(), st.gcs.buffer.gcontext(), 0, 0)?;
        Ok(())
    }

    pub fn init_file_info_font_buffer(
        conn: &C,
        st: &MeviState<C>,
        sc: &Screen,
        vi: &RenderVisualInfo,
        fi: &RenderString,
    ) -> Result<()> {
        conn.create_pixmap(
            sc.root_depth,
            st.pms.font_buffer.pixmap(),
            sc.root,
            fi.total_width,
            fi.total_height,
        )?;

        conn.render_create_picture(
            st.pics.font_buffer.picture(),
            st.pms.font_buffer.pixmap(),
            vi.root.pict_format,
            &CreatePictureAux::default()
                .polyedge(PolyEdge::SMOOTH)
                .polymode(PolyMode::IMPRECISE),
        )?;
        Ok(())
    }

    pub fn run_event_loop(&mut self) -> Result<()> {
        loop {
            let event = self.conn.wait_for_event()?;

            match MeviEvent::handle(self, event) {
                MeviEvent::DrawImage => self.needs_redraw = true,
                MeviEvent::ToggleFileInfo => self.toggle_show_file_info(),
                MeviEvent::Menu(menu_evt) => match self.menu.handle_event(menu_evt)? {
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
        mevi_info!(
            "{} file info",
            if self.show_file_info {
                "Showing"
            } else {
                "Hiding"
            }
        );
        self.needs_redraw = true;
    }

    fn draw_image(&mut self) -> Result<()> {
        let di = DrawInfo::calculate(*self.conn, &self.state, &self.image)?;
        self.w = di.parent.w;
        self.h = di.parent.h;

        // create off-screen buffer for drawing
        self.conn.create_pixmap(
            self.screen.root_depth,
            self.state.pms.buffer.pixmap(),
            self.screen.root,
            di.parent.w,
            di.parent.h,
        )?;

        self.fill_bg(&di)?;
        self.fill_back_buffer(&di)?;
        self.copy_to_window(&di)?;

        // free the off-screen buffer
        self.conn.free_pixmap(self.state.pms.buffer.pixmap())?;
        self.conn.flush()?;

        self.needs_redraw = false;
        Ok(())
    }

    pub fn fill_back_buffer(&self, di: &DrawInfo) -> Result<()> {
        self.conn.copy_area(
            self.state.pms.image.pixmap(),
            self.state.pms.buffer.pixmap(),
            self.state.gcs.buffer.gcontext(),
            di.child.x,
            di.child.y,
            di.parent.x,
            di.parent.y,
            di.child.w,
            di.child.h,
        )?;

        self.draw_file_info()?;
        Ok(())
    }

    fn fill_bg(&self, di: &DrawInfo) -> Result<()> {
        self.conn.poly_fill_rectangle(
            self.state.pms.buffer.pixmap(),
            self.state.gcs.tile.gcontext(),
            &[Rect::new(0, 0, di.parent.w, di.parent.h).into()],
        )?;

        Ok(())
    }

    fn copy_to_window(&self, di: &DrawInfo) -> Result<()> {
        self.conn.copy_area(
            self.state.pms.buffer.pixmap(),
            self.state.window.window(),
            self.state.gcs.buffer.gcontext(),
            0,
            0,
            0,
            0,
            di.parent.w,
            di.parent.h,
        )?;

        mevi_info!(
            "Copied back buffer contents from pixmap {} to window {}",
            self.state.pms.buffer.pixmap(),
            self.state.window.window()
        );

        Ok(())
    }

    fn draw_file_info(&self) -> Result<()> {
        if self.show_file_info {
            self.conn.render_create_picture(
                self.state.pics.buffer.picture(),
                self.state.pms.buffer.pixmap(),
                self.vis_info.root.pict_format,
                &CreatePictureAux::default().repeat(Repeat::NORMAL),
            )?;

            self.font_drawer.draw(
                *self.conn,
                self.state.pics.font_buffer.picture(),
                self.state.pics.buffer.picture(),
                &self.file_info,
                None,
                0,
                GRAY_RENDER_COLOR,
            )?;

            self.conn
                .render_free_picture(self.state.pics.buffer.picture())?;
        }
        Ok(())
    }
}
