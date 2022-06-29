use crate::utils::{input_pressed, Hitbox, IntVector2D};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Boomerang {
    pub hitbox: Hitbox,
    pub velocity: IntVector2D,
    pub current_animation: String,
    pub current_animation_frame: usize,
}

impl Boomerang {
    pub fn new() -> Boomerang {
        return Boomerang {
            hitbox: Hitbox {
                x: 0,
                y: 0,
                width: 8000,
                height: 8000,
            },
            velocity: IntVector2D { x: 0, y: 0 },
            current_animation: "idle".to_string(),
            current_animation_frame: 0,
        };
    }

    pub fn advance(&mut self, input: u8, prev_input: u8) {
        println!("advancing");
        self.current_animation_frame += 1;
    }
}
