use x11rb::{
    image::PixelLayout,
    protocol::xproto::{Screen, VisualClass, Visualid},
};

pub fn check_visual(screen: &Screen, id: Visualid) -> PixelLayout {
    let visual_info = screen.allowed_depths.iter().find_map(|d| {
        let info = d.visuals.iter().find(|d| d.visual_id == id);
        info.map(|i| (d.depth, i))
    });
    let (depth, visual_type) = match visual_info {
        Some(info) => info,
        None => {
            mevi_err!("Did not find the root visual's description");
            std::process::exit(1);
        }
    };

    match visual_type.class {
        VisualClass::TRUE_COLOR | VisualClass::DIRECT_COLOR => {}
        _ => {
            mevi_err!("The root visual is not true or direct color, but {visual_type:?}");
            std::process::exit(1);
        }
    };

    let pixel_layout = match PixelLayout::from_visual_type(*visual_type) {
        Ok(pl) => pl,
        Err(e) => {
            mevi_err!("The server sent a malformed visual type: {e:?}");
            std::process::exit(1);
        }
    };
    assert_eq!(pixel_layout.depth(), depth);
    pixel_layout
}
