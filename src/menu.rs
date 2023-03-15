use std::rc::Rc;

use crate::{
    event::MenuEvent,
    font::{FontDrawer, RenderLine, RenderString},
    screen::RenderVisualInfo,
    util::{Rect, StatefulRenderPicture, GRAY_RENDER_COLOR, WHITE_RENDER_COLOR},
    xy_in_rect,
};
use anyhow::Result;
use x11rb::{
    connection::Connection,
    protocol::{
        render::{ConnectionExt as _, CreatePictureAux, Picture, PolyEdge, PolyMode},
        xproto::{
            ConfigureWindowAux, ConnectionExt, CreateWindowAux, Rectangle, Screen, Window,
            WindowClass,
        },
    },
    rust_connection::RustConnection,
};

#[derive(Debug, Clone, Copy)]
pub enum MenuAction {
    ToggleFileInfo,
    Exit,
    None,
}

pub struct Menu {
    id: u32,
    pict: Picture,
    vis_info: Rc<RenderVisualInfo>,
    font_drawer: Rc<FontDrawer>,
    pub visible: bool,
    items: Vec<MenuItem>,
    pub selected: Option<usize>,
    rect: Rectangle,
}

#[derive(Debug, Clone)]
struct MenuItem {
    render_picture: StatefulRenderPicture,
    text: RenderString,
    rect: Rect,
    action: MenuAction,
}

impl MenuItem {
    fn new(
        conn: &RustConnection,
        vis_info: &RenderVisualInfo,
        parent_id: Window,
        parent_w: u16,
        text: RenderString,
        action: MenuAction,
        rect: Rect,
    ) -> Result<Self> {
        let srp =
            StatefulRenderPicture::new(conn, vis_info, parent_id, parent_w, text.total_height)?;
        Ok(Self {
            render_picture: srp,
            text,
            action,
            rect,
        })
    }

    fn set_active(&mut self) {
        self.text.fg = self.render_picture.active.fg;
        self.text.bg = self.render_picture.active.bg;
    }

    fn set_inactive(&mut self) {
        self.text.fg = self.render_picture.inactive.fg;
        self.text.bg = self.render_picture.inactive.bg;
    }

    pub fn get_picture(&mut self, selected: bool) -> Picture {
        if selected {
            self.set_active();
            self.render_picture.active.picture
        } else {
            self.set_inactive();
            self.render_picture.inactive.picture
        }
    }
}

impl Menu {
    pub fn create(
        conn: &RustConnection,
        screen: &Screen,
        parent: Window,
        vis_info: Rc<RenderVisualInfo>,
        font_drawer: Rc<FontDrawer>,
    ) -> Result<Self> {
        let id = conn.generate_id()?;
        let data = [
            (
                MenuAction::ToggleFileInfo,
                RenderString::new(
                    vec![RenderLine::new(&font_drawer, "Show file info")],
                    0,
                    WHITE_RENDER_COLOR,
                    GRAY_RENDER_COLOR,
                )
                .pad(5),
            ),
            (
                MenuAction::Exit,
                RenderString::new(
                    vec![RenderLine::new(&font_drawer, "Exit")],
                    0,
                    WHITE_RENDER_COLOR,
                    GRAY_RENDER_COLOR,
                )
                .pad(5),
            ),
        ];

        let mut total_width = 0;
        let mut total_height = 0;
        for (_, string) in &data {
            let bw = string.box_width();
            if bw > total_width {
                total_width = bw;
            }
            total_height += string.box_height();
        }
        let total_width = total_width as u16;

        conn.create_window(
            vis_info.root.depth,
            id,
            parent,
            0,
            0,
            total_width,
            total_height,
            1,
            WindowClass::INPUT_OUTPUT,
            0,
            &CreateWindowAux::default().border_pixel(screen.black_pixel),
        )?;

        let mut offset_y = 0;
        let mut menu_items = vec![];
        for (i, (action, string)) in data.into_iter().enumerate() {
            let height = string.box_height();
            let item = MenuItem::new(
                conn,
                &vis_info,
                id,
                total_width,
                string,
                action,
                Rect::new(0, i as i16 * height as i16, total_width, height),
            )?;
            offset_y += item.text.box_height() as i16;
            menu_items.push(item);
        }

        let menu = Self {
            id,
            pict: conn.generate_id()?,
            vis_info,
            visible: false,
            font_drawer,
            items: menu_items,
            selected: Some(0),
            rect: Rectangle {
                x: 0,
                y: 0,
                width: total_width,
                height: offset_y as u16,
            },
        };
        Ok(menu)
    }

    pub fn handle_event(&mut self, conn: &RustConnection, e: MenuEvent) -> Result<MenuAction> {
        let mut action: Option<MenuAction> = None;
        let needs_redraw = match e {
            MenuEvent::MapAt(x, y) => self.map_window(conn, x, y)?,
            MenuEvent::Unmap => self.unmap_window(conn)?,
            MenuEvent::Next => self.select_next(),
            MenuEvent::Prev => self.select_prev(),
            MenuEvent::FindHovered(x, y) => self.select_at_xy(x, y),
            MenuEvent::Select => {
                action = Some(self.get_action());
                self.unmap_window(conn)?
            }
            MenuEvent::Deselect => self.deselect(),
        };

        if needs_redraw {
            self.draw(conn)?;
        }

        Ok(action.unwrap_or(MenuAction::None))
    }

    fn map_window(&mut self, conn: &RustConnection, x: i16, y: i16) -> Result<bool> {
        conn.configure_window(self.id, &ConfigureWindowAux::new().x(x as i32).y(y as i32))?;
        conn.map_window(self.id)?;
        conn.flush()?;

        conn.render_create_picture(
            self.pict,
            self.id,
            self.vis_info.root.pict_format,
            &CreatePictureAux::default()
                .polyedge(PolyEdge::SMOOTH)
                .polymode(PolyMode::IMPRECISE),
        )?;

        self.rect.x = x;
        self.rect.y = y;
        self.visible = true;
        self.selected = Some(0);

        mevi_info!("Mapped menu window to pos (x: {x}, y: {y})");

        Ok(true)
    }

    fn unmap_window(&mut self, conn: &RustConnection) -> Result<bool> {
        conn.render_free_picture(self.pict)?;
        conn.unmap_window(self.id)?;
        conn.flush()?;
        self.visible = false;

        mevi_info!("Unmapped menu window");
        Ok(true)
    }

    fn draw(&mut self, conn: &RustConnection) -> Result<()> {
        if !self.visible {
            return Ok(());
        }

        let selected = self.selected.unwrap_or(usize::MAX);
        for (i, item) in self.items.iter_mut().enumerate() {
            self.font_drawer.draw(
                conn,
                item.get_picture(i == selected),
                self.pict,
                &item.text,
                Some(self.rect.width as i16),
                (item.rect.x, item.rect.y),
            )?;
        }
        conn.flush()?;
        Ok(())
    }

    pub fn select_at_xy(&mut self, x: i16, y: i16) -> bool {
        let height = self.items.len() as i16 * self.items[0].rect.h as i16;
        let (rel_x, rel_y) = (x - self.rect.x, y - self.rect.y);
        if rel_y >= height && self.selected.is_some() {
            return self.deselect();
        }

        let mut needs_redraw = false;
        for (i, item) in &mut self.items.iter_mut().enumerate() {
            let r: Rectangle = item.rect.into();
            if !xy_in_rect!(rel_x, rel_y, r) {
                continue;
            }
            if let Some(sel) = self.selected {
                needs_redraw = sel != i;
            } else {
                needs_redraw = true;
            }
            self.selected = Some(i);
        }
        needs_redraw
    }

    pub fn get_action(&self) -> MenuAction {
        if let Some(i) = self.selected {
            return self.items[i].action;
        }
        MenuAction::None
    }

    pub fn select_next(&mut self) -> bool {
        if let Some(i) = self.selected {
            if i == self.items.len() - 1 {
                self.selected = Some(0);
            } else {
                self.selected = Some(i + 1);
            }
        } else {
            self.selected = Some(0);
        }
        true
    }

    pub fn select_prev(&mut self) -> bool {
        if let Some(i) = self.selected {
            if i == 0 {
                self.selected = Some(self.items.len() - 1);
            } else {
                self.selected = Some(i - 1);
            }
        } else {
            self.selected = Some(self.items.len() - 1);
        }
        true
    }

    pub fn deselect(&mut self) -> bool {
        let needs_redraw = self.selected.is_some();
        self.selected = None;
        needs_redraw
    }

    pub fn rect(&self) -> Rectangle {
        self.rect
    }
}
