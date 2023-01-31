use anyhow::Result;
use std::borrow::Cow;
use std::env;
use x11rb::image::{BitsPerPixel, Image, ImageOrder, ScanlinePad};

pub fn get_image_from_args() -> Result<(Image<'static>, String)> {
    let file_path = if env::args_os().count() == 2 {
        env::args_os().nth(1).unwrap()
    } else {
        eprintln!("Please supply an image path");
        std::process::exit(1);
    };

    let img_buffer = image::open(&file_path)?.into_rgb8();

    let img = Image::new(
        img_buffer.width() as u16,
        img_buffer.height() as u16,
        ScanlinePad::Pad8,
        24,
        BitsPerPixel::B24,
        ImageOrder::LsbFirst,
        Cow::from(img_buffer.into_vec()),
    )?;

    Ok((img, file_path.to_string_lossy().to_string()))
}
