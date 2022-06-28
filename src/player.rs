use serde::{Deserialize, Serialize};

use crate::game::{
    INPUT_ATTACK, INPUT_DODGE, INPUT_DOWN, INPUT_JUMP, INPUT_LEFT,
    INPUT_RIGHT, INPUT_UP,
};
use crate::level::{Level, TILE_SIZE};
use crate::utils::{
    approach, clamp, do_hitboxes_overlap, input_check, input_pressed,
    input_released, Hitbox, IntVector2D,
};

// The frame rate of the original esports heaven
pub const OG_FPS: i32 = 60;

pub const RUN_ACCEL: i32 = 400 * 1000;
pub const RUN_ACCEL_TURN_MULTIPLIER: i32 = 2;
pub const RUN_DECEL: i32 = RUN_ACCEL * RUN_ACCEL_TURN_MULTIPLIER;
pub const AIR_ACCEL: i32 = 360 * 1000;
pub const AIR_DECEL: i32 = 360 * 1000;
pub const MAX_RUN_SPEED: i32 = 100 * 1000;
//pub const MAX_SUPERJUMP_SPEED_X: i32 = 250 * 1000;
//pub const MAX_SUPERJUMP_SPEED_X_OFF_WALL_SLIDE: i32 = 150 * 1000;
pub const MAX_AIR_SPEED: i32 = 120 * 1000;
pub const GRAVITY: i32 = 500 * 1000;
pub const FASTFALL_GRAVITY: i32 = 1200 * 1000;
pub const GRAVITY_ON_WALL: i32 = 150 * 1000;
pub const JUMP_POWER: i32 = 160 * 1000;
pub const JUMP_CANCEL_POWER: i32 = 40 * 1000;
pub const WALL_JUMP_POWER_X: i32 = 130 * 1000;
pub const WALL_JUMP_POWER_Y: i32 = 120 * 1000;
//pub const WALL_STICKINESS: i32 = 60 * 1000;
pub const MAX_FALL_SPEED: i32 = 270 * 1000;
pub const MAX_FALL_SPEED_ON_WALL: i32 = 200 * 1000;
pub const MAX_FASTFALL_SPEED: i32 = 500 * 1000;
pub const DOUBLE_JUMP_POWER_Y: i32 = 130 * 1000;
//pub const DODGE_DURATION = 8;
//pub const SLIDE_DURATION = 18;
//pub const SLIDE_DECEL = 100 * 1000;
//pub const DODGE_COOLDOWN = 8;
//pub const DODGE_SPEED = 260 * 1000;

#[derive(Clone, Serialize, Deserialize)]
pub struct Player {
    pub hitbox: Hitbox,
    pub velocity: IntVector2D,
    pub current_animation: String,
    pub current_animation_frame: usize,
    pub is_facing_right: bool,
    pub was_on_ground: bool,
    pub was_on_wall: bool,
    pub can_double_jump: bool,
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
            was_on_ground: true,
            was_on_wall: false,
            can_double_jump: true,
        };
    }

    pub fn advance(&mut self, input: u8, prev_input: u8, level: &Level) {
        let is_on_ground =
            self.collide(level, self.hitbox.x, self.hitbox.y + 1);
        let is_on_left_wall =
            self.collide(level, self.hitbox.x - 1, self.hitbox.y);
        let is_on_right_wall =
            self.collide(level, self.hitbox.x + 1, self.hitbox.y);
        let is_on_wall = is_on_left_wall || is_on_right_wall;

        // movement
        let mut accel = if is_on_ground { RUN_ACCEL } else { AIR_ACCEL };
        if is_on_ground
            && (input_check(INPUT_LEFT, input) && self.velocity.x > 0
                || input_check(INPUT_RIGHT, input) && self.velocity.x < 0)
        {
            accel *= RUN_ACCEL_TURN_MULTIPLIER;
        }
        let decel = if is_on_ground { RUN_DECEL } else { AIR_DECEL };
        if input_check(INPUT_LEFT, input) && !is_on_left_wall {
            self.velocity.x -= accel / OG_FPS;
        } else if input_check(INPUT_RIGHT, input) && !is_on_right_wall {
            self.velocity.x += accel / OG_FPS;
        } else if !is_on_wall {
            self.velocity.x = approach(self.velocity.x, 0, decel / OG_FPS);
        }

        let max_speed = if is_on_ground {
            MAX_RUN_SPEED
        } else {
            MAX_AIR_SPEED
        };
        self.velocity.x = clamp(self.velocity.x, -max_speed, max_speed);

        if is_on_ground {
            self.velocity.y = 0;
            self.can_double_jump = true;
            if input_pressed(INPUT_JUMP, input, prev_input) {
                self.velocity.y = -JUMP_POWER;
            }
        } else if is_on_wall {
            let gravity = if self.velocity.y > 0 {
                GRAVITY_ON_WALL
            } else {
                GRAVITY
            };
            self.velocity.y += gravity / OG_FPS;
            self.velocity.y =
                std::cmp::min(self.velocity.y, MAX_FALL_SPEED_ON_WALL);
            if input_pressed(INPUT_JUMP, input, prev_input) {
                self.velocity.y = -WALL_JUMP_POWER_Y;
                self.velocity.x = if is_on_left_wall {
                    WALL_JUMP_POWER_X
                } else {
                    -WALL_JUMP_POWER_X
                };
            }
        } else {
            if input_pressed(INPUT_JUMP, input, prev_input)
                && self.can_double_jump
            {
                self.velocity.y = -DOUBLE_JUMP_POWER_Y;
                if self.velocity.x > 0 && input_check(INPUT_LEFT, input)
                    || self.velocity.x < 0
                        && input_check(INPUT_RIGHT, input)
                {
                    self.velocity.x = 0;
                }
                self.can_double_jump = false;
            }
            if input_released(INPUT_JUMP, input, prev_input) {
                self.velocity.y =
                    std::cmp::max(self.velocity.y, -JUMP_CANCEL_POWER);
            }
            let mut gravity = GRAVITY;
            let mut max_fall_speed = MAX_FALL_SPEED;
            if input_check(INPUT_DOWN, input)
                && self.velocity.y > -JUMP_CANCEL_POWER {
                    gravity = FASTFALL_GRAVITY;
                    max_fall_speed = MAX_FASTFALL_SPEED;
            }
            self.velocity.y += gravity / OG_FPS;
            self.velocity.y =
                std::cmp::min(self.velocity.y, max_fall_speed);
        }

        self.was_on_ground = is_on_ground;
        self.was_on_wall = is_on_wall;
        // TODO: Could optimize by only sweeping
        // when player is at tunneling velocity
        self.move_by(
            level,
            self.velocity.x / OG_FPS,
            self.velocity.y / OG_FPS,
            true,
        );

        // animation
        self.current_animation_frame += 1;
        if !is_on_ground {
            if is_on_wall {
                self.set_animation("wall");
                self.is_facing_right = is_on_left_wall;
            } else {
                self.set_animation("jump");
                if input_check(INPUT_LEFT, input) {
                    self.is_facing_right = true;
                } else if input_check(INPUT_RIGHT, input) {
                    self.is_facing_right = false;
                }
            }
        } else if self.velocity.x != 0 {
            if self.velocity.x > 0 && input_check(INPUT_LEFT, input)
                || self.velocity.x < 0 && input_check(INPUT_RIGHT, input)
            {
                self.set_animation("skid");
            } else {
                self.set_animation("run");
            }
            self.is_facing_right = self.velocity.x < 0;
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
