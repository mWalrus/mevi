use anyhow::Result;
use std::{borrow::Cow, path::PathBuf};
use x11rb::image::{BitsPerPixel, Image, ImageOrder, ScanlinePad};

pub fn load_image(fp: &PathBuf) -> Result<Image<'static>> {
    let img_buf = image::open(fp)?.into_rgb8();

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
