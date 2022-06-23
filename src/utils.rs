use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntVector2D {
    pub x: i32,
    pub y: i32,
}

impl IntVector2D {
    pub fn zero(&mut self) {
        self.x = 0;
        self.y = 0;
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Hitbox {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

pub fn do_hitboxes_overlap(a: &Hitbox, b: &Hitbox) -> bool {
    let is_not_overlapping = a.x > b.x + b.width
        || b.x > a.x + a.width
        || a.y > b.y + b.height
        || b.y > a.y + a.height;
    return !is_not_overlapping;
}
