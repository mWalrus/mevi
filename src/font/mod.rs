pub mod loader;
pub mod render_string;

use self::loader::LoadedFont;
use crate::util::Rect;
use anyhow::Result;
pub use render_string::{RenderLine, RenderString, ToRenderLine};
use x11rb::{
    protocol::render::{ConnectionExt, Glyphset, PictOp, Picture},
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
        alt_width: Option<i16>,
        (x, y): (i16, i16),
    ) -> Result<()> {
        let height = string.box_height();
        let width = string.box_width();
        conn.render_fill_rectangles(
            PictOp::SRC,
            src,
            string.fg,
            &[Rect::new(
                x,
                y,
                (string.hpad + string.total_width) as u16,
                string.vpad + string.total_height,
            )
            .into()],
        )?;
        let fill_area = Rect::new(x, y, alt_width.unwrap_or(width) as u16, height);
        conn.render_fill_rectangles(PictOp::SRC, dst, string.bg, &[fill_area.into()])?;
        let mut offset_y = y;
        for line in &string.lines {
            let mut offset_x = fill_area.x + x + string.hpad;
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
