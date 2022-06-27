use serde::{Deserialize, Serialize};

use crate::game::{INPUT_DOWN, INPUT_LEFT, INPUT_RIGHT, INPUT_UP};
use crate::level::{Level, TILE_SIZE};
use crate::utils::{approach, clamp, do_hitboxes_overlap, Hitbox, input_check, IntVector2D};

pub const RUN_ACCEL: i32 = 400 * 1000;
pub const RUN_ACCEL_TURN_MULTIPLIER: i32 = 2;
pub const RUN_DECEL: i32 = RUN_ACCEL * RUN_ACCEL_TURN_MULTIPLIER;
//pub const AIR_ACCEL: i32 = 360;
//pub const AIR_DECEL: i32 = 360;
pub const MAX_RUN_SPEED: i32 = 100 * 1000;
//pub const MAX_SUPERJUMP_SPEED_X: i32 = 250;
//pub const MAX_SUPERJUMP_SPEED_X_OFF_WALL_SLIDE: i32 = 150;
//pub const MAX_AIR_SPEED: i32 = 120;
//pub const GRAVITY: i32 = 500;
//pub const FASTFALL_GRAVITY: i32 = 1200;
//pub const GRAVITY_ON_WALL: i32 = 150;
//pub const JUMP_POWER: i32 = 160;
//pub const JUMP_CANCEL_POWER: i32 = 40;
//pub const WALL_JUMP_POWER_X: i32 = 130;
//pub const WALL_JUMP_POWER_Y: i32 = 120;
//pub const WALL_STICKINESS: i32 = 60;
//pub const MAX_FALL_SPEED: i32 = 270;
//pub const MAX_FALL_SPEED_ON_WALL: i32 = 200;
//pub const MAX_FASTFALL_SPEED: i32 = 500;
//pub const DOUBLE_JUMP_POWER_Y: i32 = 130;
//pub const DODGE_DURATION = 0.13;
//pub const SLIDE_DURATION = 0.3;
//pub const SLIDE_DECEL = 100;
//pub const DODGE_COOLDOWN = 0.13;
//pub const DODGE_SPEED = 260;

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
        //self.velocity.zero();
        //if input & INPUT_UP != 0 && input & INPUT_DOWN == 0 {
            //self.velocity.y = -1777;
        //}
        //if input & INPUT_UP == 0 && input & INPUT_DOWN != 0 {
            //self.velocity.y = 1777;
        //}
        //if input & INPUT_LEFT != 0 && input & INPUT_RIGHT == 0 {
            //self.velocity.x = -1777;
            //self.is_facing_right = true;
        //}
        //if input & INPUT_LEFT == 0 && input & INPUT_RIGHT != 0 {
            //self.velocity.x = 1777;
            //self.is_facing_right = false;
        //}
        //let is_on_ground =
            //self.collide(level, self.hitbox.x, self.hitbox.y + 1);

        if input_check(input, INPUT_LEFT) {
            self.velocity.x -= RUN_ACCEL / 60;
        }
        else if input_check(input, INPUT_RIGHT) {
            self.velocity.x += RUN_ACCEL / 60;
        }
        else {
            self.velocity.x = approach(self.velocity.x, 0, RUN_DECEL);
        }
        self.velocity.x = clamp(self.velocity.x, -MAX_RUN_SPEED, MAX_RUN_SPEED);

        // TODO: Could optimize by only sweeping
        // when player is at tunneling velocity
        self.move_by(level, self.velocity.x / 60, self.velocity.y / 60, true);

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
