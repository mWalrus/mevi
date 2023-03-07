use crate::{window::GRAY_COLOR, xy_in_rect};
use anyhow::Result;
use x11rb::{
    connection::Connection,
    protocol::xproto::{
        ConfigureWindowAux, ConnectionExt, CreateGCAux, CreateWindowAux, Gcontext, Rectangle,
        Screen, WindowClass,
    },
    rust_connection::RustConnection,
};

const MENU_ITEM_HEIGHT: u16 = 20;

#[derive(Debug, Clone, Copy)]
pub enum MenuAction {
    ShowInfo,
    Exit,
    None,
}

#[derive(Debug)]
pub struct Menu {
    id: u32,
    parent_id: u32,
    depth: u8,
    bg: u32,
    fg: u32,
    font_gc1: Gcontext,
    font_gc2: Gcontext,
    gc1: Gcontext,
    gc2: Gcontext,
    pub visible: bool,
    items: [MenuItem; 2],
    pub selected: Option<usize>,
    rect: Rectangle,
}

#[derive(Debug, Clone, Copy)]
struct MenuItem {
    name: &'static str,
    rect: Rectangle,
    action: MenuAction,
}

impl MenuItem {
    fn new(
        name: &'static str,
        action: MenuAction,
        x: i16,
        y: i16,
        width: u16,
        height: u16,
    ) -> Self {
        Self {
            name,
            action,
            rect: Rectangle {
                x,
                y,
                width,
                height,
            },
        }
    }

    pub fn text_position(&self) -> i16 {
        self.rect.height as i16 - 13 / 2
    }

    fn rect(&self) -> Rectangle {
        self.rect
    }
}

impl Menu {
    pub fn create(
        conn: &RustConnection,
        screen: &Screen,
        font_gc1: Gcontext,
        font_gc2: Gcontext,
        parent_id: u32,
    ) -> Result<Self> {
        let id = conn.generate_id()?;
        let selected_gc = conn.generate_id()?;
        let normal_gc = conn.generate_id()?;

        conn.create_gc(
            selected_gc,
            parent_id,
            &CreateGCAux::default()
                .graphics_exposures(0)
                .foreground(GRAY_COLOR),
        )?;
        conn.create_gc(
            normal_gc,
            parent_id,
            &CreateGCAux::default()
                .graphics_exposures(0)
                .foreground(screen.white_pixel),
        )?;

        let width = 100;
        let items = [
            MenuItem::new(
                "Show file info",
                MenuAction::ShowInfo,
                0,
                0,
                width,
                MENU_ITEM_HEIGHT,
            ),
            MenuItem::new("Exit", MenuAction::Exit, 0, 20, width, MENU_ITEM_HEIGHT),
        ];
        let height = items.len() as u16 * MENU_ITEM_HEIGHT;

        let menu = Self {
            id,
            parent_id,
            depth: screen.root_depth,
            bg: screen.white_pixel,
            fg: screen.black_pixel,
            gc1: normal_gc,
            gc2: selected_gc,
            font_gc1,
            font_gc2,
            visible: false,
            items,
            selected: Some(0),
            rect: Rectangle {
                x: 0,
                y: 0,
                width,
                height,
            },
        };
        conn.create_window(
            menu.depth,
            menu.id,
            menu.parent_id,
            menu.rect.x,
            menu.rect.y,
            menu.rect.width,
            menu.rect.height,
            1,
            WindowClass::INPUT_OUTPUT,
            0,
            &CreateWindowAux::default()
                .background_pixel(menu.bg)
                .border_pixel(menu.fg),
        )?;

        Ok(menu)
    }

    pub fn map_window(&mut self, conn: &RustConnection, x: i16, y: i16) -> Result<()> {
        conn.configure_window(self.id, &ConfigureWindowAux::new().x(x as i32).y(y as i32))?;
        conn.map_window(self.id)?;
        conn.flush()?;

        self.rect.x = x;
        self.rect.y = y;
        self.visible = true;
        self.selected = Some(0);

        self.draw(conn)?;

        mevi_info!("Mapped menu window to pos (x: {x}, y: {y})");

        Ok(())
    }

    pub fn unmap_window(&mut self, conn: &RustConnection) -> Result<()> {
        conn.unmap_window(self.id)?;
        conn.flush()?;
        self.visible = false;

        mevi_info!("Unmapped menu window");
        Ok(())
    }

    pub fn draw(&self, conn: &RustConnection) -> Result<()> {
        let selected = self.selected.unwrap_or(usize::MAX);
        for (i, item) in self.items.iter().enumerate() {
            let (font_gc, bg_gc) = if i == selected {
                (self.font_gc2, self.gc2)
            } else {
                (self.font_gc1, self.gc1)
            };
            conn.poly_fill_rectangle(self.id, bg_gc, &[item.rect()])?;
            conn.image_text8(
                self.id,
                font_gc,
                5,
                item.rect.y + item.text_position(),
                item.name.as_bytes(),
            )?;
        }
        conn.flush()?;
        Ok(())
    }

    pub fn select_at_xy(&mut self, x: i16, y: i16) -> bool {
        let height = self.items.len() as i16 * self.items[0].rect.height as i16;
        let (rel_x, rel_y) = (x - self.rect.x, y - self.rect.y);
        if rel_y >= height && self.selected.is_some() {
            return self.deselect();
        }

        let mut needs_redraw = false;
        for (i, item) in &mut self.items.iter_mut().enumerate() {
            if !xy_in_rect!(rel_x, rel_y, item.rect()) {
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

    pub fn select_next(&mut self) {
        if let Some(i) = self.selected {
            if i == self.items.len() - 1 {
                self.selected = Some(0);
            } else {
                self.selected = Some(i + 1);
            }
            return;
        }
        self.selected = Some(0);
    }

    pub fn select_prev(&mut self) {
        if let Some(i) = self.selected {
            if i == 0 {
                self.selected = Some(self.items.len() - 1);
            } else {
                self.selected = Some(i - 1);
            }
            return;
        }
        self.selected = Some(self.items.len() - 1);
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
