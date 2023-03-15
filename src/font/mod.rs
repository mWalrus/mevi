pub mod loader;
pub mod render_string;

use self::loader::LoadedFont;
use crate::{state::MeviState, util::Rect};
use anyhow::Result;
pub use render_string::{RenderLine, RenderString, ToRenderLine};
use x11rb::{
    protocol::render::{ConnectionExt, Glyphset, PictOp},
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
        state: &MeviState,
        string: &RenderString,
        padding_x: i16,
        padding_y: u16,
    ) -> Result<()> {
        let height = string.box_height(padding_y);
        conn.render_fill_rectangles(
            PictOp::SRC,
            state.pics.font_buffer,
            string.fg,
            &[Rect::new(0, 0, (string.total_width + padding_x) as u16, height).into()],
        )?;
        let fill_area = Rect::new(0, 0, (string.total_width + (padding_x * 2)) as u16, height);
        conn.render_fill_rectangles(
            PictOp::SRC,
            state.pics.buffer,
            string.bg,
            &[fill_area.into()],
        )?;
        let mut offset_y = padding_y;
        for line in &string.lines {
            let mut offset_x = fill_area.x + padding_x;
            for chunk in &line.chunks {
                self.draw_glyphs(
                    conn,
                    offset_x,
                    offset_y as i16,
                    chunk.glyph_set,
                    state,
                    &chunk.glyph_ids,
                )?;

                offset_x += chunk.width;
            }
            offset_y += line.height + string.line_gap;
        }
        Ok(())
    }

    fn draw_glyphs(
        &self,
        conn: &RustConnection,
        x: i16,
        y: i16,
        glyphs: Glyphset,
        state: &MeviState,
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

        conn.render_composite_glyphs16(
            PictOp::OVER,
            state.pics.font_buffer,
            state.pics.buffer,
            0,
            glyphs,
            0,
            0,
            &buf,
        )?;

        Ok(())
    }
}
