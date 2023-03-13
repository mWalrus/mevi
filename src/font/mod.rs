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

    pub fn geometry(&self, text: &str) -> (i16, u16) {
        self.font.geometry(text)
    }

    pub fn draw(
        &self,
        conn: &RustConnection,
        state: &MeviState,
        string: &RenderString,
        text_x: i16,
        text_y: i16,
    ) -> Result<()> {
        conn.render_fill_rectangles(
            PictOp::SRC,
            state.pics.font_buffer,
            string.fg,
            &[Rect::new(
                0,
                text_y,
                (string.total_width + text_x) as u16,
                string.total_height,
            )
            .into()],
        )?;
        let fill_area = Rect::new(
            0,
            0,
            (string.total_width + (text_x * 2)) as u16,
            string.total_height,
        );
        conn.render_fill_rectangles(
            PictOp::SRC,
            state.pics.buffer,
            string.bg,
            &[fill_area.into()],
        )?;
        let mut offset_y = 0;
        for line in string.lines.iter() {
            let mut offset = fill_area.x + text_x;
            for chunk in &line.chunks {
                let box_shift = chunk.font_height / 2;
                self.draw_glyphs(
                    conn,
                    offset,
                    text_y + box_shift + offset_y,
                    chunk.glyph_set,
                    state,
                    &chunk.glyph_ids,
                )?;

                offset += chunk.width;
            }
            offset_y += line.height as i16;
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
