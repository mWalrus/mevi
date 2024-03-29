use std::rc::Rc;

use crate::{
    event::MenuEvent,
    font::{FontDrawer, RenderLine, RenderString},
    screen::RenderVisualInfo,
    util::{Rect, StatefulRenderPicture, GRAY_RENDER_COLOR, LIGHT_GRAY_RENDER_COLOR},
    xy_in_rect,
};
use anyhow::Result;
use x11rb::{
    connection::Connection,
    protocol::{
        render::{Color, ConnectionExt as _, CreatePictureAux, Picture, PolyEdge, PolyMode},
        xproto::{
            ConfigureWindowAux, ConnectionExt, CreateWindowAux, Rectangle, Screen, Window,
            WindowClass,
        },
    },
};

#[derive(Debug, Clone, Copy)]
pub enum MenuAction {
    ToggleFileInfo,
    Fullscreen,
    Exit,
    None,
}

pub struct Menu<'m, C: Connection> {
    id: u32,
    conn: Rc<&'m C>,
    pict: Picture,
    vis_info: Rc<RenderVisualInfo>,
    font_drawer: Rc<FontDrawer>,
    pub visible: bool,
    items: Vec<MenuItem>,
    pub selected: Option<usize>,
    pub rect: Rect,
}

#[derive(Debug, Clone)]
struct MenuItem {
    srp: StatefulRenderPicture,
    text: RenderString,
    rect: Rect,
    action: MenuAction,
}

impl MenuItem {
    fn new<C: Connection>(
        conn: &C,
        vis_info: &RenderVisualInfo,
        parent_id: Window,
        parent_w: u16,
        text: RenderString,
        action: MenuAction,
        rect: Rect,
    ) -> Result<Self> {
        let (_, h) = text.box_dimensions();
        let srp = StatefulRenderPicture::new(conn, vis_info, parent_id, parent_w, h)?;
        Ok(Self {
            srp,
            text,
            action,
            rect,
        })
    }
    pub fn get_pict_and_color(&mut self, selected: bool) -> (Picture, Color) {
        if selected {
            (self.srp.active.picture, LIGHT_GRAY_RENDER_COLOR)
        } else {
            (self.srp.inactive.picture, GRAY_RENDER_COLOR)
        }
    }
}

impl<'m, C: Connection> Menu<'m, C> {
    pub fn create(
        conn: Rc<&'m C>,
        screen: &Screen,
        parent: Window,
        vis_info: Rc<RenderVisualInfo>,
        font_drawer: Rc<FontDrawer>,
    ) -> Result<Self> {
        let id = conn.generate_id()?;
        let data = [
            (
                MenuAction::ToggleFileInfo,
                RenderString::new(vec![RenderLine::new(&font_drawer, "Show file info")]).pad(5),
            ),
            (
                MenuAction::Fullscreen,
                RenderString::new(vec![RenderLine::new(&font_drawer, "Fullscreen")]).pad(5),
            ),
            (
                MenuAction::Exit,
                RenderString::new(vec![RenderLine::new(&font_drawer, "Exit")]).pad(5),
            ),
        ];

        let mut total_width = 0;
        let mut total_height = 0;
        for (_, string) in &data {
            let (w, h) = string.box_dimensions();
            if w > total_width {
                total_width = w;
            }
            total_height += h;
        }
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
        for (action, string) in data {
            let (_, h) = string.box_dimensions();
            let item = MenuItem::new(
                *conn,
                &vis_info,
                id,
                total_width,
                string,
                action,
                Rect::new(0, offset_y, total_width, h),
            )?;
            offset_y += h as i16;
            info!("Constructed menu item with rect: {:?}", item.rect);
            menu_items.push(item);
        }
        info!("Total menu dimensions: width -> {total_width}px, height -> {total_height}px");

        let pict = conn.generate_id()?;

        let menu = Self {
            id,
            conn,
            pict,
            vis_info,
            visible: false,
            font_drawer,
            items: menu_items,
            selected: Some(0),
            rect: Rect::new(0, 0, total_width, offset_y as u16),
        };
        info!("Constructed the menu");
        Ok(menu)
    }

    pub fn handle_event(&mut self, e: MenuEvent) -> Result<MenuAction> {
        let mut action: Option<MenuAction> = None;
        let needs_redraw = match e {
            MenuEvent::MapAt(x, y) => self.map_window(x, y)?,
            MenuEvent::Unmap => self.unmap_window()?,
            MenuEvent::Next => self.select_next(),
            MenuEvent::Prev => self.select_prev(),
            MenuEvent::FindHovered(x, y) => self.select_at_xy(x, y),
            MenuEvent::Select => {
                action = Some(self.get_action());
                self.unmap_window()?
            }
            MenuEvent::Deselect => self.deselect(),
        };

        if needs_redraw {
            self.draw()?;
        }

        Ok(action.unwrap_or(MenuAction::None))
    }

    fn map_window(&mut self, x: i16, y: i16) -> Result<bool> {
        self.conn
            .configure_window(self.id, &ConfigureWindowAux::new().x(x as i32).y(y as i32))?;
        self.conn.map_window(self.id)?;
        self.conn.flush()?;

        self.conn.render_create_picture(
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

        info!("Mapped menu window to pos (x: {x}, y: {y})");

        Ok(true)
    }

    fn unmap_window(&mut self) -> Result<bool> {
        self.conn.render_free_picture(self.pict)?;
        self.conn.unmap_window(self.id)?;
        self.conn.flush()?;
        self.visible = false;

        info!("Unmapped menu window");
        Ok(true)
    }

    fn draw(&mut self) -> Result<()> {
        if !self.visible {
            return Ok(());
        }

        let selected = self.selected.unwrap_or(usize::MAX);
        for (i, item) in self.items.iter_mut().enumerate() {
            info!("Redrawing menu item {}", i + 1);
            let (pict, color) = item.get_pict_and_color(i == selected);
            self.font_drawer.draw(
                *self.conn,
                pict,
                self.pict,
                &item.text,
                Some(self.rect.w),
                item.rect.y,
                color,
            )?;
        }
        self.conn.flush()?;
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
}
