use anyhow::Result;
use image::imageops::FilterType;
use image::io::Reader as ImageReader;
use lazy_static::{__Deref, lazy_static};
use std::{borrow::Cow, fmt::Debug, fs::File, path::PathBuf};
use x11rb::{
    connection::Connection,
    image::{BitsPerPixel, ColorComponent, Image, ImageOrder, PixelLayout, ScanlinePad},
    protocol::xproto::Screen,
    rust_connection::RustConnection,
};

use crate::font::{FontDrawer, RenderLine, ToRenderLine};

lazy_static! {
    static ref FOREIGN_PIXEL_LAYOUT: PixelLayout = PixelLayout::new(
        ColorComponent::new(8, 0).unwrap(),
        ColorComponent::new(8, 8).unwrap(),
        ColorComponent::new(8, 16).unwrap(),
    );
}

pub struct MeviImage {
    pub inner: Image<'static>,
    pub ow: u32,
    pub oh: u32,
    pub w: u16,
    pub h: u16,
    pub size: u64,
    pub path: String,
    pub format: String,
}

impl Debug for MeviImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MeviImage {{ ow: {}, oh: {}, w: {}, h: {}, size: {}, path: {}. format: {} }}",
            self.ow, self.oh, self.w, self.h, self.size, self.path, self.format
        )
    }
}

impl MeviImage {
    pub fn new(
        conn: &RustConnection,
        screen: &Screen,
        path: &PathBuf,
        pixel_layout: PixelLayout,
    ) -> Result<Self> {
        let size = {
            let f = File::open(path)?;
            let data = f.metadata()?;
            data.len() / 1024 // Kb
        };

        let image = ImageReader::open(path)?.with_guessed_format()?;
        let format = if let Some(fmt) = image.format() {
            format!("{fmt:?}")
        } else {
            "unknown".into()
        };
        let mut image = image.decode()?;
        let (ow, oh) = (image.width(), image.height());

        let (sw, sh) = (
            screen.width_in_pixels as u32,
            screen.height_in_pixels as u32,
        );
        if ow > sw || oh > sh {
            image = image.resize(sw, sh, FilterType::Lanczos3)
        }

        let image_buffer = image.into_rgb8();
        let (new_w, new_h) = (image_buffer.width() as u16, image_buffer.height() as u16);

        let image = Image::new(
            new_w,
            new_h,
            ScanlinePad::Pad8,
            24,
            BitsPerPixel::B24,
            ImageOrder::LsbFirst,
            Cow::from(image_buffer.into_vec()),
        )?;

        let image = image.reencode(*FOREIGN_PIXEL_LAYOUT, pixel_layout, conn.setup())?;

        let mevi_image = MeviImage {
            inner: image.deref().to_owned(),
            ow,
            oh,
            w: new_w,
            h: new_h,
            size,
            path: path.to_str().unwrap().to_owned(),
            format,
        };
        mevi_info!("Loaded image: {mevi_image:?}");

        Ok(mevi_image)
    }
}

impl ToRenderLine for MeviImage {
    fn to_lines(&self, font_drawer: &FontDrawer) -> Vec<RenderLine> {
        vec![
            RenderLine::new(font_drawer, format!("path: {}", self.path)),
            RenderLine::new(font_drawer, format!("dimensions: {}x{}", self.ow, self.oh)),
            RenderLine::new(font_drawer, format!("type: {}", self.format)),
            RenderLine::new(font_drawer, format!("size: {}Kb", self.size)),
        ]
    }
}

pub fn get_bg_image(conn: &RustConnection, pixel_layout: PixelLayout) -> Result<Image<'static>> {
    let bytes = include_bytes!("../resources/transparent-bg-smaller.png");

    let image_buffer = image::load_from_memory(bytes)?.into_rgb8();
    let image = Image::new(
        image_buffer.width() as u16,
        image_buffer.height() as u16,
        ScanlinePad::Pad8,
        24,
        BitsPerPixel::B24,
        ImageOrder::LsbFirst,
        Cow::from(image_buffer.into_vec()),
    )?;

    let image = image.reencode(*FOREIGN_PIXEL_LAYOUT, pixel_layout, conn.setup())?;
    mevi_info!("Loaded background image");

    Ok(image.deref().to_owned())
}
