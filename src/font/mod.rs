pub mod loader;
pub mod render_string;

use self::loader::LoadedFont;
use crate::util::Rect;
use anyhow::Result;
pub use render_string::{RenderLine, RenderString, ToRenderLine};
use x11rb::{
    protocol::{
        render::{ConnectionExt, Glyphset, PictOp, Picture},
        xproto::Rectangle,
    },
    rust_connection::RustConnection,
};

pub struct FontDrawer {
    font: LoadedFont,
}

impl FontDrawer {
    pub fn new(font: LoadedFont) -> Self {
        Self { font }
    }

    pub fn draw(
        &self,
        conn: &RustConnection,
        src: Picture,
        dst: Picture,
        string: &RenderString,
        alt_width: Option<u16>,
        y: i16,
    ) -> Result<()> {
        let (w, h) = string.box_dimensions();
        let w = alt_width.unwrap_or(w);

        let fg_fill_area: Rectangle = Rect::new(0, 0, w, h).into();
        let bg_fill_area: Rectangle = Rect::new(0, y, w, h).into();

        conn.render_fill_rectangles(PictOp::SRC, src, string.fg, &[fg_fill_area])?;
        conn.render_fill_rectangles(PictOp::SRC, dst, string.bg, &[bg_fill_area])?;

        let mut offset_y = y;
        for line in &string.lines {
            let mut offset_x = string.hpad as i16;
            for chunk in &line.chunks {
                self.draw_glyphs(
                    conn,
                    (offset_x, offset_y),
                    chunk.glyph_set,
                    src,
                    dst,
                    &chunk.glyph_ids,
                )?;

                offset_x += chunk.width;
            }
            offset_y += (line.height + string.line_gap) as i16;
        }
        Ok(())
    }

    fn draw_glyphs(
        &self,
        conn: &RustConnection,
        (x, y): (i16, i16),
        glyphs: Glyphset,
        src: Picture,
        dst: Picture,
        glyph_ids: &[u32],
    ) -> Result<()> {
        let mut buf = Vec::with_capacity(glyph_ids.len());
        let render = if glyph_ids.len() > 254 {
            &glyph_ids[..254]
        } else {
            glyph_ids
        };

        buf.extend_from_slice(&[render.len() as u8, 0, 0, 0]);

        buf.extend_from_slice(&(x).to_ne_bytes());
        buf.extend_from_slice(&(y).to_ne_bytes());

        for glyph in render {
            buf.extend_from_slice(&(glyph).to_ne_bytes());
        }

        conn.render_composite_glyphs16(PictOp::OVER, src, dst, 0, glyphs, 0, 0, &buf)?;

        Ok(())
    }
}
