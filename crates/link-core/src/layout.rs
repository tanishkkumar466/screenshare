use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type MachineId = Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect { pub x: i32, pub y: i32, pub width: u32, pub height: u32 }

impl Rect {
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.width as i32 && y < self.y + self.height as i32
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Display {
    pub id: String,
    pub machine: MachineId,
    pub name: String,
    pub bounds: Rect,
    pub scale: f32,
    pub refresh_hz: u16,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DisplayLayout { pub displays: Vec<Display> }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge { Left, Right, Top, Bottom }

impl DisplayLayout {
    /// Finds the display bordering `from` in the requested direction. Overlap is required.
    pub fn neighbor(&self, from: &str, edge: Edge) -> Option<&Display> {
        let source = self.displays.iter().find(|d| d.id == from)?;
        self.displays.iter().find(|target| {
            if target.id == source.id { return false; }
            match edge {
                Edge::Left => target.bounds.x + target.bounds.width as i32 == source.bounds.x
                    && overlaps(target.bounds.y, target.bounds.height, source.bounds.y, source.bounds.height),
                Edge::Right => source.bounds.x + source.bounds.width as i32 == target.bounds.x
                    && overlaps(target.bounds.y, target.bounds.height, source.bounds.y, source.bounds.height),
                Edge::Top => target.bounds.y + target.bounds.height as i32 == source.bounds.y
                    && overlaps(target.bounds.x, target.bounds.width, source.bounds.x, source.bounds.width),
                Edge::Bottom => source.bounds.y + source.bounds.height as i32 == target.bounds.y
                    && overlaps(target.bounds.x, target.bounds.width, source.bounds.x, source.bounds.width),
            }
        })
    }
}

fn overlaps(a: i32, alen: u32, b: i32, blen: u32) -> bool {
    a < b + blen as i32 && b < a + alen as i32
}
