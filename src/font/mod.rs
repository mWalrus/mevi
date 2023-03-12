pub mod loader;

use self::loader::{FontEncodedChunk, LoadedFont};
use crate::{state::MeviState, util::Rect};
use anyhow::Result;
use x11rb::{
    protocol::render::{Color, ConnectionExt, Glyphset, PictOp},
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
            &[Rect::new(1, 1, (string.width + text_x) as u16, string.height).into()],
        )?;
        let fill_area = Rect::new(0, 0, (string.width + (text_x * 2)) as u16, string.height);
        conn.render_fill_rectangles(
            PictOp::SRC,
            state.pics.buffer,
            string.bg,
            &[fill_area.into()],
        )?;
        let mut offset = fill_area.x + text_x;
        for chunk in &string.chunks {
            let box_shift = (fill_area.h as i16 - chunk.font_height) / 2;
            self.draw_glyphs(
                conn,
                offset,
                text_y + box_shift,
                chunk.glyph_set,
                state,
                &chunk.glyph_ids,
            )?;

            offset += chunk.width;
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

pub struct RenderString {
    pub text: String,
    pub chunks: Vec<FontEncodedChunk>,
    pub width: i16,
    pub height: u16,
    pub bg: Color,
    pub fg: Color,
}

impl RenderString {
    pub fn new(drawer: &FontDrawer, text: impl ToString, bg: Color, fg: Color) -> Self {
        let text = text.to_string();
        let (width, height) = drawer.geometry(&text);
        let chunks = drawer.font.encode(&text, width - 1);
        Self {
            text: text.to_string(),
            chunks,
            width,
            height,
            bg,
            fg,
        }
    }
}
