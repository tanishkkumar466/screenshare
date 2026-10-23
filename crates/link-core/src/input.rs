use crate::layout::{DisplayLayout, Edge, MachineId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PointerMotion { pub dx: i32, pub dy: i32 }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InputEvent {
    PointerMotion(PointerMotion),
    Button { button: u8, pressed: bool },
    Scroll { horizontal: i16, vertical: i16 },
    Key { scan_code: u16, pressed: bool },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Route { Local, Remote(MachineId) }

/// Keeps keyboard ownership with the display currently owning the pointer.
pub struct InputRouter { pub active_display: String, cursor_x: i32, cursor_y: i32 }

impl InputRouter {
    pub fn new(display: impl Into<String>, x: i32, y: i32) -> Self {
        Self { active_display: display.into(), cursor_x: x, cursor_y: y }
    }

    pub fn route(&mut self, layout: &DisplayLayout, event: &InputEvent) -> Route {
        if let InputEvent::PointerMotion(delta) = event {
            self.cursor_x += delta.dx;
            self.cursor_y += delta.dy;
            let edge = match (delta.dx.signum(), delta.dy.signum()) {
                (-1, _) => Some(Edge::Left), (1, _) => Some(Edge::Right), (_, -1) => Some(Edge::Top), (_, 1) => Some(Edge::Bottom), _ => None,
            };
            if let Some(edge) = edge {
                if let Some(next) = layout.neighbor(&self.active_display, edge) {
                    if next.bounds.contains(self.cursor_x, self.cursor_y) {
                        self.active_display = next.id.clone();
                        self.cursor_x = self.cursor_x.clamp(next.bounds.x, next.bounds.x + next.bounds.width as i32 - 1);
                        self.cursor_y = self.cursor_y.clamp(next.bounds.y, next.bounds.y + next.bounds.height as i32 - 1);
                        return Route::Remote(next.machine);
                    }
                }
            }
        }
        match layout.displays.iter().find(|d| d.id == self.active_display) {
            Some(display) => Route::Remote(display.machine),
            None => Route::Local,
        }
    }
}
