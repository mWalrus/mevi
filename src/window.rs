use crate::{screen, Atoms, CLI};
use anyhow::Result;
use std::borrow::Cow;
use std::fmt::Display;
use x11rb::connection::Connection;
use x11rb::image::{ColorComponent, Image, PixelLayout};
use x11rb::protocol::xproto::{
    ConnectionExt, CreateGCAux, CreateWindowAux, EventMask, FillStyle, Gcontext, Pixmap, PropMode,
    Rectangle, Screen, Window, WindowClass,
};
use x11rb::protocol::Event;
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;

pub static INITIAL_SIZE: (u16, u16) = (600, 800);
pub static TITLE: &str = "mevi";

pub struct DrawInfo {
    pub child: Rectangle,
    pub parent: Rectangle,
}

impl Display for DrawInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "draw info: (parent: {:x?}, child: {:x?})",
            self.parent, self.child
        )
    }
}

struct ImageInfo {
    original_width: u32,
    original_height: u32,
    width: u16,
    height: u16,
    path: String,
}

pub struct Mevi<'a> {
    atoms: Atoms,
    conn: &'a RustConnection,
    window: Window,
    screen: &'a Screen,
    buffer: Pixmap,
    buffer_gc: Gcontext,
    image_pixmap: Pixmap,
    image_info: ImageInfo,
    tile_gc: Gcontext,
    font_gc: Gcontext,
    show_menu: bool,
}

impl<'a> Mevi<'a> {
    pub fn init(
        conn: &'a RustConnection,
        screen: &'a Screen,
        atoms: Atoms,
        image: &'a Image,
        orig_w: u32,
        orig_h: u32,
        bg_img: &'a Image,
    ) -> Result<Self> {
        let window = conn.generate_id()?;
        let image_pixmap = conn.generate_id()?;
        let buffer = conn.generate_id()?;
        let buffer_gc = conn.generate_id()?;
        let background_pixmap = conn.generate_id()?;
        let background_gc = conn.generate_id()?;
        let tile_gc = conn.generate_id()?;
        let font_gc = conn.generate_id()?;
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

        let (img, bg) = Self::reencode_images(&conn, screen, &image, &bg_img)?;

        let image_info = ImageInfo {
            original_width: orig_w,
            original_height: orig_h,
            width: img.width(),
            height: img.height(),
            path,
        };

        bg.put(conn, background_pixmap, background_gc, 0, 0)?;

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
            image_info.width,
            image_info.height,
        )?;

        img.put(conn, image_pixmap, buffer_gc, 0, 0)?;

        let win_aux = CreateWindowAux::default()
            .event_mask(EventMask::EXPOSURE | EventMask::STRUCTURE_NOTIFY);

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

        Ok(Self {
            atoms,
            conn,
            window,
            screen,
            buffer,
            buffer_gc,
            image_pixmap,
            image_info,
            tile_gc,
            font_gc,
            show_menu: false,
        })
    }

    pub fn run_event_handler(&self) -> Result<()> {
        loop {
            let event = self.conn.wait_for_event()?;

            match event {
                Event::Expose(e) if e.count == 0 => {
                    mevi_event!(e);
                    self.draw()?;
                }
                Event::ClientMessage(evt) => {
                    let data = evt.data.as_data32();
                    if evt.format == 32
                        && evt.window == self.window
                        && data[0] == self.atoms.WM_DELETE_WINDOW
                    {
                        mevi_info!("Exit signal received");
                        break;
                    }
                }
                Event::Error(e) => mevi_err!("Received error: {e:?}"),
                _ => {}
            }
        }
        Ok(())
    }

    fn draw(&self) -> Result<()> {
        let info = self.calc_image_draw_info()?;

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

        if CLI.info || CLI.debug {
            self.conn.image_text8(
                self.buffer,
                self.font_gc,
                0,
                11,
                format!(
                    "path: {} | dimensions: {}x{}",
                    self.image_info.path,
                    self.image_info.original_width,
                    self.image_info.original_height
                )
                .as_bytes(),
            )?;
        }

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
        Ok(())
    }

    fn reencode_images(
        conn: &RustConnection,
        screen: &Screen,
        image: &'a Image,
        bg: &'a Image,
    ) -> Result<(Cow<'a, Image<'a>>, Cow<'a, Image<'a>>)> {
        let foreign_layout = PixelLayout::new(
            ColorComponent::new(8, 0)?,
            ColorComponent::new(8, 8)?,
            ColorComponent::new(8, 16)?,
        );

        let pixel_layout = screen::check_visual(screen, screen.root_visual);

        let img = image.reencode(foreign_layout, pixel_layout, conn.setup())?;

        let bg = bg.reencode(foreign_layout, pixel_layout, conn.setup())?;
        Ok((img, bg))
    }

    fn calc_image_draw_info(&self) -> Result<DrawInfo> {
        let attrs = self.conn.get_geometry(self.window)?.reply()?;
        let (parent_w, parent_h) = (attrs.width, attrs.height);
        let (cx, cy) = (parent_w as i16 / 2, parent_h as i16 / 2);

        let child_x = cx - (self.image_info.width as i16 / 2);
        let child_y = cy - (self.image_info.height as i16 / 2);

        let (child_x, parent_x, child_w) = if self.image_info.width > parent_w {
            (child_x.abs(), 0, parent_w)
        } else {
            (0, child_x, self.image_info.width)
        };
        let (child_y, parent_y, child_h) = if self.image_info.height > parent_h {
            (child_y.abs(), 0, parent_h)
        } else {
            (0, child_y, self.image_info.height)
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
