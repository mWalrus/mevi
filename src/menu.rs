use anyhow::Result;
use x11rb::{
    connection::Connection,
    protocol::xproto::{
        ConfigureWindowAux, ConnectionExt, CoordMode, CreateWindowAux, Gcontext, Pixmap, Point,
        Rectangle, Screen, WindowClass,
    },
    rust_connection::RustConnection,
};

#[derive(Debug)]
pub struct Menu {
    id: u32,
    parent_id: u32,
    depth: u8,
    bg: u32,
    fg: u32,
    pub visible: bool,
    items: [MenuItem; 2],
    render_offset: i16,
    rect: Rectangle,
}

#[derive(Debug)]
struct MenuItem {
    name: &'static str,
    rect: Rectangle,
}

impl MenuItem {
    fn new(name: &'static str, x: i16, y: i16, width: u16, height: u16) -> Self {
        Self {
            name,
            rect: Rectangle {
                x,
                y,
                width,
                height: 13,
            },
        }
    }

    fn collides_with(&self, pointer: (i16, i16), px: i16, py: i16) -> bool {
        let (x, y) = pointer;
        let within_width = x > px + self.rect.x && x < px + self.rect.x + self.rect.width as i16;
        let within_height = y > py + self.rect.y && y < py + self.rect.y + self.rect.height as i16;
        within_height && within_width
    }
}

impl Menu {
    pub fn create(conn: &RustConnection, screen: &Screen, parent_id: u32) -> Result<Self> {
        let id = conn.generate_id()?;

        let (width, height) = (100, 150);

        let menu = Self {
            id,
            parent_id,
            depth: screen.root_depth,
            bg: screen.white_pixel,
            fg: screen.black_pixel,
            visible: false,
            items: [
                MenuItem::new("Show file info", 0, 10, width, 20),
                MenuItem::new("Test item", 0, 30, width, 20),
            ],
            render_offset: 20,
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

        mevi_info!("Mapped menu window to pos (x: {x}, y: {y})");

        Ok(())
    }

    pub fn draw(
        &self,
        conn: &RustConnection,
        pointer: (i16, i16),
        gc: Gcontext,
        gc_sel: Gcontext,
    ) -> Result<()> {
        for (i, item) in self.items.iter().enumerate() {
            let offset = self.render_offset * (i as i16 + 1);
            if item.collides_with(pointer, self.rect.x, self.rect.y) {
                conn.image_text8(self.id, gc_sel, 5, offset, item.name.as_bytes())?;
            } else {
                conn.image_text8(self.id, gc, 5, offset, item.name.as_bytes())?;
            }
        }
        Ok(())
    }

    pub fn unmap_window(&mut self, conn: &RustConnection) -> Result<()> {
        conn.unmap_window(self.id)?;
        conn.flush()?;
        self.visible = false;

        mevi_info!("Unmapped menu window");
        Ok(())
    }

    pub fn has_pointer_within(&self, px: i16, py: i16) -> bool {
        let over_x = px > self.rect.x && px < self.rect.x + self.rect.width as i16;
        let over_y = py > self.rect.y && py < self.rect.y + self.rect.height as i16;
        over_x && over_y && self.visible
    }
}
