use x11rb::protocol::render::Color;

use super::{loader::FontEncodedChunk, FontDrawer};

#[derive(Debug, Clone)]
pub struct RenderLine {
    pub chunks: Vec<FontEncodedChunk>,
    pub width: i16,
    pub height: u16,
}

impl RenderLine {
    pub fn new(drawer: &FontDrawer, text: impl ToString) -> Self {
        let text = text.to_string();
        let (width, height) = drawer.font.geometry(&text);
        let chunks = drawer.font.encode(&text, width - 1);
        Self {
            chunks,
            width,
            height,
        }
    }
}

pub trait ToRenderLine {
    fn to_lines(&self, font_drawer: &FontDrawer) -> Vec<RenderLine>;
}

#[derive(Debug, Clone)]
pub struct RenderString {
    pub lines: Vec<RenderLine>,
    pub line_gap: u16,
    pub total_width: u16,
    pub total_height: u16,
    pub vpad: u16,
    pub hpad: u16,
    pub bg: Color,
    pub fg: Color,
}

impl RenderString {
    pub fn new(lines: Vec<RenderLine>, line_gap: u16, bg: Color, fg: Color) -> Self {
        let mut total_width = 0;
        for line in lines.iter() {
            if line.width > total_width {
                total_width = line.width;
            }
        }
        let total_height = lines[0].height * lines.len() as u16;
        Self {
            lines,
            line_gap,
            total_width: total_width as u16,
            total_height,
            vpad: 0,
            hpad: 0,
            bg,
            fg,
        }
    }

    pub fn pad(mut self, pad: u16) -> Self {
        self.hpad = pad;
        self.vpad = pad;
        self
    }

    pub fn box_dimensions(&self) -> (u16, u16) {
        (
            self.total_width + (self.hpad * 2),
            self.total_height + ((self.lines.len() as u16 - 1) * self.line_gap) + (self.vpad * 2),
        )
    }
}
