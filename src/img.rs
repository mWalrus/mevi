use anyhow::Result;
use std::borrow::Cow;
use std::env;
use x11rb::image::{BitsPerPixel, Image, ImageOrder, ScanlinePad};

pub fn get_image_from_args() -> Result<(Image<'static>, String)> {
    let fp = if env::args_os().count() == 2 {
        env::args_os().nth(1).unwrap()
    } else {
        eprintln!("Please supply an image path");
        std::process::exit(1);
    };

    let img_buf = image::open(&fp)?.into_rgb8();

    let img = Image::new(
        img_buf.width() as u16,
        img_buf.height() as u16,
        ScanlinePad::Pad8,
        24,
        BitsPerPixel::B24,
        ImageOrder::LsbFirst,
        Cow::from(img_buf.into_vec()),
    )?;

    Ok((img, fp.to_string_lossy().to_string()))
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
