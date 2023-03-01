use anyhow::Result;
use image::imageops::FilterType;
use std::{borrow::Cow, path::PathBuf};
use x11rb::image::{BitsPerPixel, Image, ImageOrder, ScanlinePad};

pub struct ImageWrapper {
    pub image: Image<'static>,
    pub path: String,
    pub width: u16,
    pub height: u16,
}

pub fn load_image(fp: &PathBuf, sw: u32, sh: u32) -> Result<ImageWrapper> {
    let mut img = image::open(fp)?;

    if img.width() > sw || img.height() > sh {
        img = img.resize(sw / 2, sh / 2, FilterType::Nearest)
    }

    let img_buffer = img.into_rgb8();
    let width = img_buffer.width() as u16;
    let height = img_buffer.height() as u16;

    let image = Image::new(
        width,
        height,
        ScanlinePad::Pad8,
        24,
        BitsPerPixel::B24,
        ImageOrder::LsbFirst,
        Cow::from(img_buffer.into_vec()),
    )?;

    let wrapper = ImageWrapper {
        image,
        path: fp.to_string_lossy().to_string(),
        width,
        height,
    };

    Ok(wrapper)
}

pub fn get_bg_image() -> Result<Image<'static>> {
    let bytes = include_bytes!("resources/transparent-bg-smaller.png");

    let img_buf = image::load_from_memory(bytes)?.into_rgb8();
    let img = Image::new(
        img_buf.width() as u16,
        img_buf.height() as u16,
        ScanlinePad::Pad8,
        24,
        BitsPerPixel::B24,
        ImageOrder::LsbFirst,
        Cow::from(img_buf.into_vec()),
    )?;

    Ok(img)
}
