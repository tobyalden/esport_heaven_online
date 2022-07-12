use crate::utils::IntVector2D;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Particle {
    pub position: IntVector2D,
    pub current_animation: String,
    pub current_animation_frame: usize,
}

impl Particle {
    pub fn new() -> Particle {
        return Particle {
            position: IntVector2D { x: 10000, y: 10000 },
            current_animation: "none".to_string(),
            current_animation_frame: 0,
        };
    }

    pub fn advance(&mut self) {
        self.current_animation_frame += 1;
        // TODO: This is a big flaw in how data is organized...
        if self.current_animation_frame >= 4 * 5 {
            self.set_animation("none");
        }
    }

    pub fn set_animation(&mut self, new_animation: &str) {
        let old_animation = self.current_animation.clone();
        self.current_animation = new_animation.to_string();
        if old_animation != self.current_animation {
            self.current_animation_frame = 0;
        }
    }
}
