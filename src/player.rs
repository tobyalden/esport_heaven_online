use serde::{Deserialize, Serialize};

use crate::game::{do_hitboxes_overlap, IntVector2D, Hitbox, Level, TILE_SIZE};

#[derive(Clone, Serialize, Deserialize)]
pub struct Player {
    pub hitbox: Hitbox,
    pub velocity: IntVector2D,
}

impl Player {
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
        let tile_height = (player_hitbox.height + TILE_SIZE - 1) / TILE_SIZE;
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

