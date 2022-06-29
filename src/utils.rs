use fixed::types::I32F32;
use fixed_sqrt::FixedSqrt;
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

    pub fn length(&self) -> I32F32 {
        let length = I32F32::from_num(self.x * self.x + self.y * self.y);
        return length.sqrt();
    }

    pub fn normalize(&mut self, size: i32) {
        if !(self.x == 0 && self.y == 0) {
            let normal =
                I32F32::from_num(size).saturating_div(self.length());
            let new_x = I32F32::from_num(self.x).saturating_mul(normal);
            let new_y = I32F32::from_num(self.y).saturating_mul(normal);
            self.x = new_x.saturating_to_num::<i32>();
            self.y = new_y.saturating_to_num::<i32>();
        }
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
    } else if value > target + amount {
        return value - amount;
    } else {
        return target;
    }
}

pub fn clamp(value: i32, min: i32, max: i32) -> i32 {
    if max > min {
        if value < min {
            return min;
        } else if value > max {
            return max;
        } else {
            return value;
        }
    } else {
        if value < max {
            return max;
        } else if value > min {
            return min;
        } else {
            return value;
        }
    }
}

pub fn input_check(check: u8, input: u8) -> bool {
    return input & check != 0;
}

pub fn input_pressed(check: u8, input: u8, prev_input: u8) -> bool {
    return input_check(check, input) && !input_check(check, prev_input);
}

pub fn input_released(check: u8, input: u8, prev_input: u8) -> bool {
    return !input_check(check, input) && input_check(check, prev_input);
}
