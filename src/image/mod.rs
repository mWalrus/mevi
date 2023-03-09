use anyhow::Result;
use image::imageops::FilterType;
use std::{borrow::Cow, path::PathBuf};
use x11rb::image::{BitsPerPixel, Image, ImageOrder, ScanlinePad};

pub fn load_image(fp: &PathBuf, sw: u32, sh: u32) -> Result<(Image<'static>, u32, u32)> {
    let mut image = image::open(fp)?;
    let (orig_width, orig_height) = (image.width(), image.height());

    if image.width() > sw || image.height() > sh {
        image = image.resize(sw, sh, FilterType::Lanczos3)
    }

    let image_buffer = image.into_rgb8();

    let image = Image::new(
        image_buffer.width() as u16,
        image_buffer.height() as u16,
        ScanlinePad::Pad8,
        24,
        BitsPerPixel::B24,
        ImageOrder::LsbFirst,
        Cow::from(image_buffer.into_vec()),
    )?;

    Ok((image, orig_width, orig_height))
}

pub fn get_bg_image() -> Result<Image<'static>> {
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

    Ok(image)
}
