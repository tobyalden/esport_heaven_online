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

pub fn approach(value: i32, target: i32, amount: i32) -> i32 {
    if value < target - amount {
        return value + amount;
    }
    else if value > target + amount {
        return value - amount;
    }
    else {
        return target;
    }
}

pub fn clamp(value: i32, min: i32, max: i32) -> i32 {
    if max > min {
        if value < min { return min; }
        else if value > max { return max; }
        else { return value; }
    }
    else {
        if value < max { return max; }
        else if value > min { return min; }
        else { return value; }
    }
}

pub fn input_check(check: u8, input: u8) -> bool {
    return input & check != 0;
}
