use serde::{Deserialize, Serialize};

use crate::game::{INPUT_DOWN, INPUT_LEFT, INPUT_RIGHT, INPUT_UP};
use crate::level::{Level, TILE_SIZE};
use crate::utils::{do_hitboxes_overlap, Hitbox, IntVector2D};

#[derive(Clone, Serialize, Deserialize)]
pub struct Player {
    pub hitbox: Hitbox,
    pub velocity: IntVector2D,
    pub current_animation: String,
    pub current_animation_frame: usize,
    pub is_facing_right: bool,
}

impl Player {
    pub fn new(x: i32, y: i32, is_facing_right: bool) -> Player {
        return Player {
            hitbox: Hitbox {
                x,
                y,
                width: 6000,
                height: 12000,
            },
            velocity: IntVector2D { x: 0, y: 0 },
            current_animation: "idle".to_string(),
            current_animation_frame: 0,
            is_facing_right,
        };
    }

    pub fn advance(&mut self, input: u8, level: &Level) {
        self.velocity.zero();
        if input & INPUT_UP != 0 && input & INPUT_DOWN == 0 {
            self.velocity.y = -1777;
        }
        if input & INPUT_UP == 0 && input & INPUT_DOWN != 0 {
            self.velocity.y = 1777;
        }
        if input & INPUT_LEFT != 0 && input & INPUT_RIGHT == 0 {
            self.velocity.x = -1777;
            self.is_facing_right = true;
        }
        if input & INPUT_LEFT == 0 && input & INPUT_RIGHT != 0 {
            self.velocity.x = 1777;
            self.is_facing_right = false;
        }
        // TODO: Could optimize by only sweeping
        // when player is at tunneling velocity
        self.move_by(level, self.velocity.x, self.velocity.y, true);

        self.current_animation_frame += 1;
        if self.velocity.x != 0 {
            self.set_animation("run");
        } else {
            self.set_animation("idle");
        }
    }

    pub fn set_animation(&mut self, new_animation: &str) {
        let old_animation = self.current_animation.clone();
        self.current_animation = new_animation.to_string();
        if old_animation != self.current_animation {
            self.current_animation_frame = 0;
        }
    }

    pub fn move_by(
        &mut self,
        level: &Level,
        move_x: i32,
        move_y: i32,
        sweep: bool,
    ) {
        if sweep
            || self.collide(level, self.hitbox.x + move_x, self.hitbox.y)
        {
            let sign = if move_x > 0 { 1 } else { -1 };
            let increments = [1000, 100, 10, 1];
            let mut increment_index = 0;
            let mut move_amount = move_x.abs();
            while increment_index < increments.len() {
                while !self.collide(
                    level,
                    self.hitbox.x + increments[increment_index] * sign,
                    self.hitbox.y,
                ) && move_amount >= increments[increment_index]
                {
                    self.hitbox.x += increments[increment_index] * sign;
                    move_amount -= increments[increment_index];
                }
                increment_index += 1;
            }
        } else {
            self.hitbox.x += move_x;
        }

        if sweep
            || self.collide(level, self.hitbox.x, self.hitbox.y + move_y)
        {
            let sign = if move_y > 0 { 1 } else { -1 };
            let increments = [1000, 100, 10, 1];
            let mut increment_index = 0;
            let mut move_amount = move_y.abs();
            while increment_index < increments.len() {
                while !self.collide(
                    level,
                    self.hitbox.x,
                    self.hitbox.y + increments[increment_index] * sign,
                ) && move_amount >= increments[increment_index]
                {
                    self.hitbox.y += increments[increment_index] * sign;
                    move_amount -= increments[increment_index];
                }
                increment_index += 1;
            }
        } else {
            self.hitbox.y += move_y;
        }
    }

    pub fn collide(
        &self,
        level: &Level,
        virtual_x: i32,
        virtual_y: i32,
    ) -> bool {
        let player_hitbox = Hitbox {
            x: virtual_x,
            y: virtual_y,
            width: self.hitbox.width,
            height: self.hitbox.height,
        };
        let tile_x = virtual_x / TILE_SIZE;
        let tile_y = virtual_y / TILE_SIZE;
        // We use (dividend + divisor - 1) / divisor here
        // to get integer division that rounds up
        let tile_width = (player_hitbox.width + TILE_SIZE - 1) / TILE_SIZE;
        let tile_height =
            (player_hitbox.height + TILE_SIZE - 1) / TILE_SIZE;
        for check_x in 0..(tile_width + 1) {
            for check_y in 0..(tile_height + 1) {
                if level.check_grid(tile_x + check_x, tile_y + check_y) {
                    let grid_hitbox = Hitbox {
                        x: (tile_x + check_x) * TILE_SIZE,
                        y: (tile_y + check_y) * TILE_SIZE,
                        width: TILE_SIZE,
                        height: TILE_SIZE,
                    };
                    if do_hitboxes_overlap(&player_hitbox, &grid_hitbox) {
                        return true;
                    }
                }
            }
        }
        return false;
    }
}
