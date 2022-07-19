use crate::utils::IntVector2D;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Particle {
    pub position: IntVector2D,
    pub velocity: IntVector2D,
    pub current_animation: String,
    pub current_animation_frame: usize,
}

pub const GROUND_DUST_ANIMATION_SPEED: usize = 4;
pub const GROUND_DUST_ANIMATION_FRAMES: usize = 5;
pub const SIMPLE_ANIMATION_SPEED: usize = 10;
pub const SIMPLE_ANIMATION_FRAMES: usize = 5;

impl Particle {
    pub fn new() -> Particle {
        return Particle {
            position: IntVector2D { x: 0, y: 0 },
            velocity: IntVector2D { x: 0, y: 0 },
            current_animation: "none".to_string(),
            current_animation_frame: 0,
        };
    }

    pub fn advance(&mut self) {
        self.current_animation_frame += 1;
        // TODO: This is a big flaw in how data is organized...
        //
        if self.current_animation == "grounddust" {
            if self.current_animation_frame
                >= GROUND_DUST_ANIMATION_SPEED
                    * GROUND_DUST_ANIMATION_FRAMES
            {
                self.set_animation("none");
            }
        } else if self.current_animation == "simple" {
            if self.current_animation_frame
                >= SIMPLE_ANIMATION_SPEED * SIMPLE_ANIMATION_FRAMES
            {
                self.set_animation("none");
            }
        }

        self.position.x += self.velocity.x;
        self.position.y += self.velocity.y;
    }

    pub fn set_animation(&mut self, new_animation: &str) {
        let old_animation = self.current_animation.clone();
        self.current_animation = new_animation.to_string();
        if old_animation != self.current_animation {
            self.current_animation_frame = 0;
            self.velocity.zero();
        }
    }
}
