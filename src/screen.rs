use anyhow::Result;
use smallmap::Map;
use thiserror::Error;
use x11rb::{
    connection::Connection,
    image::PixelLayout,
    protocol::{
        render::{query_pict_formats, Directformat, PictType, Pictformat, Pictforminfo},
        xproto::{Screen, VisualClass, Visualid, Visualtype},
    },
    rust_connection::{ConnectionError, ParseError, ReplyError},
};

#[derive(Error, Debug)]
pub enum VisualError {
    #[error("Failed to query for pict formats: {0}")]
    QueryError(#[from] ConnectionError),
    #[error("Failed to get a query reply: {0}")]
    ReplyError(#[from] ReplyError),
    #[error("No appropriate visual found")]
    NoAppropriateVisual,
    #[error("The server sent a malformed visual type: {0:?}")]
    Malformed(#[from] ParseError),
    #[error("The root visual is not true or direct color, but {0:?}")]
    NotTrueOrDirect(Visualtype),
}

#[derive(Debug)]
pub struct RenderVisualInfo {
    pub root: VisualInfo,
    pub render: VisualInfo,
}

#[derive(Debug)]
pub struct VisualInfo {
    pub id: Visualid,
    pub pict_format: Pictformat,
    pub direct_format: Directformat,
    pub depth: u8,
}

impl RenderVisualInfo {
    pub fn new<C: Connection>(conn: &C, screen: &Screen) -> Result<Self, VisualError> {
        let rvi = Self {
            root: VisualInfo::find_appropriate_visual(
                conn,
                screen.root_depth,
                Some(screen.root_visual),
            )?,
            render: VisualInfo::find_appropriate_visual(conn, 32, None)?,
        };
        info!("Found appropriate visuals: {rvi:?}");
        Ok(rvi)
    }
}

impl VisualInfo {
    pub fn find_appropriate_visual<C: Connection>(
        conn: &C,
        depth: u8,
        id: Option<Visualid>,
    ) -> Result<VisualInfo, VisualError> {
        let formats = query_pict_formats(conn)?.reply()?;
        let candidates = formats
            .formats
            .into_iter()
            .filter_map(|pfi| {
                (pfi.type_ == PictType::DIRECT && pfi.depth == depth).then_some((pfi.id, pfi))
            })
            .collect::<Map<Pictformat, Pictforminfo>>();
        for screen in formats.screens {
            let candidate = screen
                .depths
                .into_iter()
                .find_map(|pd| {
                    (pd.depth == depth).then(|| {
                        pd.visuals.into_iter().find(|pv| {
                            if let Some(match_vid) = id {
                                pv.visual == match_vid && candidates.contains_key(&pv.format)
                            } else {
                                candidates.contains_key(&pv.format)
                            }
                        })
                    })
                })
                .flatten();
            if let Some(c) = candidate {
                info!("Found pict visual for visual: {c:?}");
                return Ok(VisualInfo {
                    id: c.visual,
                    pict_format: c.format,
                    direct_format: candidates[&c.format].direct,
                    depth,
                });
            }
        }
        Err(VisualError::NoAppropriateVisual)
    }
}

pub fn pixel_layout_from_visual(screen: &Screen, id: Visualid) -> Result<PixelLayout, VisualError> {
    let visual_info = screen.allowed_depths.iter().find_map(|d| {
        let info = d.visuals.iter().find(|d| d.visual_id == id);
        info.map(|i| (d.depth, i))
    });
    let (depth, visual_type) = match visual_info {
        Some(info) => info,
        None => Err(VisualError::NoAppropriateVisual)?,
    };

    match visual_type.class {
        VisualClass::TRUE_COLOR | VisualClass::DIRECT_COLOR => {}
        _ => Err(VisualError::NotTrueOrDirect(*visual_type))?,
    };
    let pixel_layout = PixelLayout::from_visual_type(*visual_type)?;
    assert_eq!(pixel_layout.depth(), depth);
    info!("Found pixel layout from visual {id}: {pixel_layout:?}");
    Ok(pixel_layout)
}
