use fixed::types::I32F32;
use fixed_macro::fixed;
use serde::{Deserialize, Serialize};

use crate::game::{
    INPUT_DODGE, INPUT_DOWN, INPUT_JUMP, INPUT_LEFT, INPUT_RIGHT, INPUT_UP,
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
pub const MAX_SUPERJUMP_SPEED_X: i32 = 250 * 1000;
pub const MAX_SUPERJUMP_SPEED_X_OFF_WALL_SLIDE: i32 = 150 * 1000;
pub const MAX_AIR_SPEED: i32 = 120 * 1000;
pub const GRAVITY: i32 = 500 * 1000;
pub const FASTFALL_GRAVITY: i32 = 1200 * 1000;
pub const GRAVITY_ON_WALL: i32 = 150 * 1000;
pub const JUMP_POWER: i32 = 160 * 1000;
pub const JUMP_CANCEL_POWER: i32 = 40 * 1000;
pub const WALL_JUMP_POWER_X: i32 = 130 * 1000;
pub const WALL_JUMP_POWER_Y: i32 = 120 * 1000;
pub const SUPER_WALL_JUMP_POWER_X: i32 = 74286;
pub const SUPER_WALL_JUMP_POWER_Y: i32 = 210000;
pub const WALL_STICKINESS: i32 = 60 * 1000;
pub const MAX_FALL_SPEED: i32 = 270 * 1000;
pub const MAX_FALL_SPEED_ON_WALL: i32 = 200 * 1000;
pub const MAX_FASTFALL_SPEED: i32 = 500 * 1000;
pub const DOUBLE_JUMP_POWER_Y: i32 = 130 * 1000;
pub const DODGE_DURATION: i32 = 9;
pub const SLIDE_DURATION: i32 = 19;
pub const SLIDE_DECEL: i32 = 100 * 1000;
pub const DODGE_COOLDOWN: i32 = 9;
pub const DODGE_SPEED: i32 = 260 * 1000;

#[derive(Clone, Serialize, Deserialize)]
pub struct Player {
    pub hitbox: Hitbox,
    pub velocity: IntVector2D,
    pub current_animation: String,
    pub current_animation_frame: usize,
    pub is_facing_left: bool,
    pub was_on_ground: bool,
    pub was_on_wall: bool,
    pub can_double_jump: bool,
    pub can_dodge: bool,
    pub dodge_timer: i32,
    pub dodge_timer_duration: i32,
    pub dodge_cooldown: i32,
    pub is_sliding: bool,
    pub is_wall_sliding: bool,
    pub is_super_jumping: bool,
    pub is_super_jumping_off_wall_slide: bool,
    pub collided_with_boomerang: bool,
    pub collided_with_player: bool,
}

impl Player {
    pub fn new(x: i32, y: i32, is_facing_left: bool) -> Player {
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
            is_facing_left,
            was_on_ground: true,
            was_on_wall: false,
            can_double_jump: true,
            can_dodge: true,
            dodge_timer: 0,
            dodge_timer_duration: DODGE_COOLDOWN,
            dodge_cooldown: 0,
            is_sliding: false,
            is_wall_sliding: false,
            is_super_jumping: false,
            is_super_jumping_off_wall_slide: false,
            collided_with_boomerang: false,
            collided_with_player: false,
        };
    }

    pub fn advance(
        &mut self,
        input: u8,
        prev_input: u8,
        level: &Level,
        other_player_hitbox: &Hitbox,
        other_boomerang_hitbox: &Hitbox,
    ) {
        let is_on_ground =
            self.collide(level, self.hitbox.x, self.hitbox.y + 1);
        let mut is_on_left_wall =
            self.collide(level, self.hitbox.x - 1, self.hitbox.y);
        let mut is_on_right_wall =
            self.collide(level, self.hitbox.x + 1, self.hitbox.y);
        let mut is_on_wall = is_on_left_wall || is_on_right_wall;

        self.collided_with_player = false;
        self.collided_with_boomerang = false;

        if self.dodge_timer > 0 {
            self.dodge_movement(
                input,
                prev_input,
                level,
                is_on_ground,
                is_on_left_wall,
                is_on_right_wall,
                is_on_wall,
                other_player_hitbox,
                other_boomerang_hitbox,
            );
        } else {
            self.movement(
                input,
                prev_input,
                level,
                is_on_ground,
                is_on_left_wall,
                is_on_right_wall,
                is_on_wall,
                other_player_hitbox,
                other_boomerang_hitbox,
            );
        }

        is_on_left_wall =
            self.collide(level, self.hitbox.x - 1, self.hitbox.y);
        is_on_right_wall =
            self.collide(level, self.hitbox.x + 1, self.hitbox.y);
        is_on_wall = is_on_left_wall || is_on_right_wall;

        if self.is_wall_sliding && !is_on_wall {
            self.is_wall_sliding = false;
            if self.was_on_wall && self.velocity.y <= 0 {
                self.velocity.y = -JUMP_CANCEL_POWER * 2;
            }
        }

        // animation
        self.current_animation_frame += 1;
        if !is_on_ground {
            if is_on_wall {
                self.set_animation("wall");
                self.is_facing_left = is_on_left_wall;
            } else {
                self.set_animation("jump");
                if input_check(INPUT_LEFT, input) {
                    self.is_facing_left = true;
                } else if input_check(INPUT_RIGHT, input) {
                    self.is_facing_left = false;
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
            self.is_facing_left = self.velocity.x < 0;
        } else {
            self.set_animation("idle");
        }

        // tick timers
        let prev_dodge_timer = self.dodge_timer;
        self.dodge_timer = approach(self.dodge_timer, 0, 1);
        self.dodge_cooldown = approach(self.dodge_cooldown, 0, 1);

        if self.dodge_timer == 0 && prev_dodge_timer > 0 {
            if self.is_sliding {
                self.is_sliding = false;
            } else if self.is_wall_sliding {
                self.is_wall_sliding = false;
            } else if self.velocity.y < 0 {
                self.velocity.y = -JUMP_CANCEL_POWER;
            } else if self.velocity.y > 0 {
                self.velocity.y = MAX_FALL_SPEED / 2;
            }
            self.dodge_cooldown = DODGE_COOLDOWN;
        }
    }

    pub fn dodge_movement(
        &mut self,
        input: u8,
        prev_input: u8,
        level: &Level,
        is_on_ground: bool,
        is_on_left_wall: bool,
        is_on_right_wall: bool,
        is_on_wall: bool,
        other_player_hitbox: &Hitbox,
        other_boomerang_hitbox: &Hitbox,
    ) {
        if self.is_sliding {
            let mut gravity = GRAVITY;
            if input_check(INPUT_DOWN, input)
                && self.velocity.y > -JUMP_CANCEL_POWER
            {
                gravity = FASTFALL_GRAVITY;
            }
            self.velocity.y += gravity / OG_FPS;
            self.velocity.y =
                std::cmp::min(self.velocity.y, MAX_FASTFALL_SPEED);
            self.velocity.x =
                approach(self.velocity.x, 0, SLIDE_DECEL / OG_FPS);

            if input_pressed(INPUT_JUMP, input, prev_input) {
                // ugly fixed point math
                let numerator = I32F32::from_num(self.dodge_timer);
                let denominator =
                    I32F32::from_num(self.dodge_timer_duration);
                let jump_modifier = fixed!(0.75: I32F32).saturating_add(
                    fixed!(0.5: I32F32).saturating_mul(
                        numerator.saturating_div(denominator),
                    ),
                );
                let new_velocity_y = I32F32::from_num(-JUMP_POWER)
                    .saturating_div(jump_modifier);
                self.velocity.y =
                    new_velocity_y.saturating_to_num::<i32>();
                let new_velocity_x = I32F32::from_num(self.velocity.x)
                    .saturating_mul(jump_modifier);
                self.velocity.x =
                    new_velocity_x.saturating_to_num::<i32>();
                self.dodge_timer = 0;
                self.is_sliding = false;
                if numerator.saturating_div(denominator) > 0.5 {
                    self.is_super_jumping = true;
                }
            }
        } else if self.is_wall_sliding {
            let mut gravity = GRAVITY;
            if input_check(INPUT_DOWN, input)
                && self.velocity.y > -JUMP_CANCEL_POWER
            {
                gravity = FASTFALL_GRAVITY;
            }
            self.velocity.y += gravity / OG_FPS;
            self.velocity.y =
                std::cmp::min(self.velocity.y, MAX_FASTFALL_SPEED);
            if input_pressed(INPUT_JUMP, input, prev_input) {
                if self.velocity.y < 0 {
                    self.velocity.y = -SUPER_WALL_JUMP_POWER_Y;
                }
                self.velocity.x = if is_on_left_wall {
                    SUPER_WALL_JUMP_POWER_X
                } else {
                    -SUPER_WALL_JUMP_POWER_X
                };
                self.dodge_timer = 0;
                self.is_wall_sliding = false;
                self.is_super_jumping = true;
                self.is_super_jumping_off_wall_slide = true;
            }
        }
        self.was_on_ground = is_on_ground;
        self.was_on_wall = is_on_wall;
        self.move_by(
            level,
            self.velocity.x / OG_FPS,
            self.velocity.y / OG_FPS,
            true,
            is_on_ground,
            is_on_left_wall,
            is_on_right_wall,
            other_player_hitbox,
            other_boomerang_hitbox,
        );
    }

    pub fn movement(
        &mut self,
        input: u8,
        prev_input: u8,
        level: &Level,
        is_on_ground: bool,
        is_on_left_wall: bool,
        is_on_right_wall: bool,
        is_on_wall: bool,
        other_player_hitbox: &Hitbox,
        other_boomerang_hitbox: &Hitbox,
    ) {
        if input_pressed(INPUT_DODGE, input, prev_input)
            && self.dodge_timer == 0
            && self.dodge_cooldown == 0
            && self.can_dodge
        {
            // Start dodging
            let mut dodge_heading = IntVector2D { x: 1, y: 0 };
            if self.is_facing_left {
                dodge_heading.x = -1;
            }
            if input_check(INPUT_LEFT, input) {
                dodge_heading.x = -1;
            } else if input_check(INPUT_RIGHT, input) {
                dodge_heading.x = 1;
            } else if input_check(INPUT_UP, input)
                || input_check(INPUT_DOWN, input)
            {
                dodge_heading.x = 0;
            }
            if input_check(INPUT_UP, input) {
                dodge_heading.y = -1;
            } else if input_check(INPUT_DOWN, input) {
                dodge_heading.y = 1;
            }

            if input_check(INPUT_DOWN, input) {
                self.reset_dodge_timer(SLIDE_DURATION);
                self.is_sliding = true;
            } else if is_on_left_wall && dodge_heading.x < 0
                || is_on_right_wall && dodge_heading.x > 0
            {
                dodge_heading.y *= 2;
                self.reset_dodge_timer(DODGE_DURATION);
                self.is_wall_sliding = true;
            } else {
                self.reset_dodge_timer(DODGE_DURATION);
                self.is_sliding = false;
            }

            // Normalize to dodge speed
            self.velocity = dodge_heading;
            self.velocity.normalize(DODGE_SPEED);
            self.can_dodge = false;
            return;
        }

        if is_on_ground || is_on_wall {
            self.is_super_jumping = false;
            self.is_super_jumping_off_wall_slide = false;
        }

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

        let mut max_speed = if is_on_ground {
            MAX_RUN_SPEED
        } else {
            MAX_AIR_SPEED
        };
        if self.is_super_jumping {
            if self.is_super_jumping_off_wall_slide {
                max_speed = MAX_SUPERJUMP_SPEED_X_OFF_WALL_SLIDE;
            } else {
                max_speed = MAX_SUPERJUMP_SPEED_X;
            }
        }
        self.velocity.x = clamp(self.velocity.x, -max_speed, max_speed);

        if is_on_ground {
            self.can_double_jump = true;
            self.can_dodge = true;
            self.velocity.y = 0;
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
            if input_released(INPUT_JUMP, input, prev_input)
                && !self.is_super_jumping
            {
                self.velocity.y =
                    std::cmp::max(self.velocity.y, -JUMP_CANCEL_POWER);
            }
            let mut gravity = GRAVITY;
            let mut max_fall_speed = MAX_FALL_SPEED;
            if input_check(INPUT_DOWN, input)
                && self.velocity.y > -JUMP_CANCEL_POWER
                && !self.is_super_jumping
            {
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
            is_on_ground,
            is_on_left_wall,
            is_on_right_wall,
            other_player_hitbox,
            other_boomerang_hitbox,
        );
    }

    pub fn reset_dodge_timer(&mut self, new_duration: i32) {
        self.dodge_timer = new_duration;
        self.dodge_timer_duration = new_duration;
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
        is_on_ground: bool,
        is_on_left_wall: bool,
        is_on_right_wall: bool,
        other_player_hitbox: &Hitbox,
        other_boomerang_hitbox: &Hitbox,
    ) {
        let mut collided_on_x = false;
        if sweep
            || self.collide(level, self.hitbox.x + move_x, self.hitbox.y)
        {
            let sign = if move_x > 0 { 1 } else { -1 };
            let increments = [1000, 100, 10, 1];
            let mut increment_index = 0;
            let mut move_amount = move_x.abs();
            while increment_index < increments.len() {
                while move_amount >= increments[increment_index] {
                    if self.collide(
                        level,
                        self.hitbox.x + increments[increment_index] * sign,
                        self.hitbox.y,
                    ) {
                        collided_on_x = true;
                        break;
                    } else {
                        self.hitbox.x +=
                            increments[increment_index] * sign;
                        move_amount -= increments[increment_index];
                        self.check_entity_collisions(
                            other_player_hitbox,
                            other_boomerang_hitbox,
                        );
                    }
                }
                increment_index += 1;
            }
        } else {
            self.hitbox.x += move_x;
        }

        if collided_on_x {
            self.move_collide_x(
                is_on_ground,
                is_on_left_wall,
                is_on_right_wall,
            );
        }

        let mut collided_on_y = false;
        if sweep
            || self.collide(level, self.hitbox.x, self.hitbox.y + move_y)
        {
            let sign = if move_y > 0 { 1 } else { -1 };
            let increments = [1000, 100, 10, 1];
            let mut increment_index = 0;
            let mut move_amount = move_y.abs();
            while increment_index < increments.len() {
                while move_amount >= increments[increment_index] {
                    if self.collide(
                        level,
                        self.hitbox.x,
                        self.hitbox.y + increments[increment_index] * sign,
                    ) {
                        collided_on_y = true;
                        break;
                    } else {
                        self.hitbox.y +=
                            increments[increment_index] * sign;
                        move_amount -= increments[increment_index];
                        self.check_entity_collisions(
                            other_player_hitbox,
                            other_boomerang_hitbox,
                        );
                    }
                }
                increment_index += 1;
            }
        } else {
            self.hitbox.y += move_y;
        }
        if collided_on_y {
            self.move_collide_y();
        }
        self.check_entity_collisions(
            other_player_hitbox,
            other_boomerang_hitbox,
        );
    }

    pub fn check_entity_collisions(
        &mut self,
        other_player_hitbox: &Hitbox,
        other_boomerang_hitbox: &Hitbox,
    ) {
        if do_hitboxes_overlap(&self.hitbox, other_player_hitbox) {
            self.collided_with_player = true;
        }
        if do_hitboxes_overlap(&self.hitbox, other_boomerang_hitbox) {
            self.collided_with_boomerang = true;
        }
    }

    pub fn move_collide_x(
        &mut self,
        is_on_ground: bool,
        is_on_left_wall: bool,
        is_on_right_wall: bool,
    ) {
        if is_on_ground {
            self.velocity.x = 0;
        } else if is_on_left_wall {
            self.velocity.x =
                std::cmp::max(self.velocity.x, -WALL_STICKINESS);
            if self.dodge_timer > 0 {
                self.is_wall_sliding = true;
            }
        } else if is_on_right_wall {
            self.velocity.x =
                std::cmp::min(self.velocity.x, WALL_STICKINESS);
            if self.dodge_timer > 0 {
                self.is_wall_sliding = true;
            }
        }
    }

    pub fn move_collide_y(&mut self) {
        self.velocity.y = 0;
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

    pub fn center_x(&self) -> i32 {
        return self.hitbox.x + self.hitbox.width / 2;
    }

    pub fn center_y(&self) -> i32 {
        return self.hitbox.y + self.hitbox.height / 2;
    }
}
