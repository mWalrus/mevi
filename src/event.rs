use x11rb::{
    connection::Connection,
    protocol::{xproto::Rectangle, Event},
    x11_utils::X11Error,
};

use crate::{app::Mevi, keys::Key};

pub enum MeviEvent {
    DrawImage,
    ToggleFileInfo,
    ToggleFullscreen,
    Menu(MenuEvent),
    Exit,
    Idle,
    Error(X11Error),
}

pub enum MenuEvent {
    MapAt(i16, i16),
    Unmap,
    Next,
    Prev,
    FindHovered(i16, i16),
    Select,
    Deselect,
}

impl MeviEvent {
    pub fn handle<C: Connection>(app: &Mevi<C>, event: Event) -> Self {
        let menu_rect: Rectangle = app.menu.rect.into();
        mevi_event!(event);
        match event {
            Event::Expose(e) if e.count == 0 => Self::DrawImage,
            Event::KeyRelease(e) => match Key::from(e.detail) {
                Key::F => Self::ToggleFullscreen,
                Key::I => Self::ToggleFileInfo,
                Key::M if !app.menu.visible => {
                    let x = (app.w / 2).saturating_sub(menu_rect.width / 2);
                    let y = (app.h / 2).saturating_sub(menu_rect.height / 2);
                    Self::Menu(MenuEvent::MapAt(x as i16, y as i16))
                }
                Key::M => Self::Menu(MenuEvent::Unmap),
                Key::Up => Self::Menu(MenuEvent::Prev),
                Key::Down => Self::Menu(MenuEvent::Next),
                Key::Esc if app.menu.visible => Self::Menu(MenuEvent::Unmap),
                Key::Esc => Self::Exit,
                Key::Enter => Self::Menu(MenuEvent::Select),
                _ => Self::Idle,
            },
            Event::ButtonPress(e) => {
                if e.detail == 3 && !app.menu.visible {
                    Self::Menu(MenuEvent::MapAt(e.event_x, e.event_y))
                } else if e.detail == 1
                    && app.menu.visible
                    && xy_in_rect!(e.event_x, e.event_y, menu_rect)
                {
                    Self::Menu(MenuEvent::Select)
                } else if (e.detail == 1 || e.detail == 3) && app.menu.visible {
                    Self::Menu(MenuEvent::Unmap)
                } else {
                    Self::Idle
                }
            }
            Event::MotionNotify(e) => {
                if app.menu.visible && xy_in_rect!(e.event_x, e.event_y, menu_rect) {
                    Self::Menu(MenuEvent::FindHovered(e.event_x, e.event_y))
                } else if app.menu.visible {
                    Self::Menu(MenuEvent::Deselect)
                } else {
                    Self::Idle
                }
            }
            Event::ClientMessage(e) => {
                let data = e.data.as_data32();
                if e.format == 32
                    && e.window == app.state.window.window()
                    && data[0] == app.atoms.WM_DELETE_WINDOW
                {
                    return Self::Exit;
                }
                Self::Idle
            }
            Event::Error(e) => Self::Error(e),
            _ => Self::Idle,
        }
    }
}
