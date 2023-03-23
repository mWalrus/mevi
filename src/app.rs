use std::fmt::Debug;
use std::rc::Rc;

use crate::event::MeviEvent;
use crate::font::loader::LoadedFont;
use crate::font::{FontDrawer, RenderString, ToRenderLine};
use crate::img::MeviImage;
use crate::menu::{Menu, MenuAction};
use crate::screen::RenderVisualInfo;
use crate::state::MeviState;
use crate::util::{Rect, GRAY_RENDER_COLOR, INITIAL_SIZE, TITLE};
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
    pub menu: Menu<'a, C>,
    pub w: u16,
    pub h: u16,
}

impl<'a, C: Connection + Debug> Mevi<'a, C> {
    pub fn init(
        conn: &'a C,
        screen: &'a Screen,
        atoms: Atoms,
        image: MeviImage,
        bg_img: Image,
    ) -> Result<Self> {
        let mut state = MeviState::init(conn)?;
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

        let wid = state.window.window();

        conn.create_window(
            screen.root_depth,
            wid,
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

        info!("Created main window");

        conn.change_property8(
            PropMode::REPLACE,
            wid,
            atoms.WM_NAME,
            atoms.STRING,
            title.as_bytes(),
        )?;

        conn.change_property8(
            PropMode::REPLACE,
            wid,
            atoms._NET_WM_NAME,
            atoms.UTF8_STRING,
            title.as_bytes(),
        )?;

        conn.change_property32(
            PropMode::REPLACE,
            wid,
            atoms.WM_PROTOCOLS,
            atoms.ATOM,
            &[atoms.WM_DELETE_WINDOW],
        )?;

        if CLI.fullscreen {
            conn.change_property32(
                PropMode::REPLACE,
                wid,
                atoms._NET_WM_STATE,
                atoms.ATOM,
                &[atoms._NET_WM_STATE_FULLSCREEN],
            )?;
            state.fullscreen = true;
        }

        info!("Set main window properties");

        conn.map_window(wid)?;
        conn.flush()?;
        info!("Mapped the main window");

        let conn = Rc::new(conn);
        let menu = Menu::create(
            Rc::clone(&conn),
            screen,
            wid,
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
            menu,
            w: INITIAL_SIZE.0,
            h: INITIAL_SIZE.1,
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
                MeviEvent::DrawImage => self.state.should_redraw = true,
                MeviEvent::ToggleFileInfo => self.toggle_show_file_info(),
                MeviEvent::ToggleFullscreen => self.toggle_fullscreen()?,
                MeviEvent::Menu(menu_evt) => match self.menu.handle_event(menu_evt)? {
                    MenuAction::ToggleFileInfo => self.toggle_show_file_info(),
                    MenuAction::Fullscreen => self.toggle_fullscreen()?,
                    MenuAction::Exit => self.state.should_exit = true,
                    MenuAction::None => {}
                },
                MeviEvent::Exit => self.state.should_exit = true,
                MeviEvent::Error(e) => err!("{e:?}"),
                MeviEvent::Idle => {}
            }

            if self.state.should_exit {
                info!("Exit signal received");
                break;
            }

            if self.state.should_redraw {
                self.draw_image()?;
            }
        }
        Ok(())
    }

    fn toggle_fullscreen(&mut self) -> Result<()> {
        let wid = self.state.window.window();

        let data = if self.state.fullscreen {
            vec![]
        } else {
            vec![self.atoms._NET_WM_STATE_FULLSCREEN]
        };

        self.conn.unmap_window(wid)?;
        self.conn.change_property32(
            PropMode::REPLACE,
            wid,
            self.atoms._NET_WM_STATE,
            self.atoms.ATOM,
            &data,
        )?;
        self.conn.map_window(wid)?;
        self.conn.flush()?;

        self.state.fullscreen = !self.state.fullscreen;

        info!(
            "Changed property _NET_WM_STATE ({}) of window {} to {:?}",
            self.atoms._NET_WM_STATE, wid, data
        );

        Ok(())
    }

    fn toggle_show_file_info(&mut self) {
        self.state.draw_info = !self.state.draw_info;
        info!(
            "{} file info",
            if self.state.draw_info {
                "Showing"
            } else {
                "Hiding"
            }
        );
        self.state.should_redraw = true;
    }

    pub fn calculate_rects(&mut self) -> Result<(Rect, Rect)> {
        let attrs = self
            .conn
            .get_geometry(self.state.window.window())?
            .reply()?;
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

        let parent = Rect::new(parent_x, parent_y, parent_w, parent_h);
        let child = Rect::new(child_x, child_y, child_w, child_h);

        self.w = parent.w;
        self.h = parent.h;

        info!("Calculated parent draw info: {parent:?}");
        info!("Calculated child draw info: {child:?}");

        Ok((parent, child))
    }

    fn draw_image(&mut self) -> Result<()> {
        let (parent, child) = self.calculate_rects()?;

        self.conn.create_pixmap(
            self.screen.root_depth,
            self.state.pms.buffer.pixmap(),
            self.screen.root,
            self.w,
            self.h,
        )?;

        self.fill_bg()?;
        self.fill_back_buffer(parent, child)?;
        self.copy_to_window()?;

        self.conn.free_pixmap(self.state.pms.buffer.pixmap())?;
        self.conn.flush()?;

        self.state.should_redraw = false;
        Ok(())
    }

    pub fn fill_back_buffer(&self, parent_rect: Rect, child_rect: Rect) -> Result<()> {
        self.conn.copy_area(
            self.state.pms.image.pixmap(),
            self.state.pms.buffer.pixmap(),
            self.state.gcs.buffer.gcontext(),
            child_rect.x,
            child_rect.y,
            parent_rect.x,
            parent_rect.y,
            child_rect.w,
            child_rect.h,
        )?;

        self.draw_file_info()?;
        Ok(())
    }

    fn fill_bg(&self) -> Result<()> {
        self.conn.poly_fill_rectangle(
            self.state.pms.buffer.pixmap(),
            self.state.gcs.tile.gcontext(),
            &[Rect::new(0, 0, self.w, self.h).into()],
        )?;

        Ok(())
    }

    fn copy_to_window(&self) -> Result<()> {
        self.conn.copy_area(
            self.state.pms.buffer.pixmap(),
            self.state.window.window(),
            self.state.gcs.buffer.gcontext(),
            0,
            0,
            0,
            0,
            self.w,
            self.h,
        )?;

        info!(
            "Copied back buffer contents from pixmap {} to window {}",
            self.state.pms.buffer.pixmap(),
            self.state.window.window()
        );

        Ok(())
    }

    fn draw_file_info(&self) -> Result<()> {
        if self.state.draw_info {
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
