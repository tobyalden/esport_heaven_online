use fixed::types::I32F32;
use fixed_macro::fixed;
use serde::{Deserialize, Serialize};

use crate::game::{
    INPUT_ATTACK, INPUT_DOWN, INPUT_LEFT, INPUT_RIGHT, INPUT_UP,
};
use crate::player::{Player, OG_FPS};
use crate::utils::{
    do_hitboxes_overlap, input_check, input_pressed, lerp, Hitbox,
    IntVector2D,
};

pub const MAX_SPEED: i32 = 300 * 1000;
pub const RETURN_RATE: I32F32 = fixed!(0.75: I32F32);

#[derive(Clone, Serialize, Deserialize)]
pub struct Boomerang {
    pub hitbox: Hitbox,
    pub velocity: IntVector2D,
    pub initial_velocity: IntVector2D,
    pub current_animation: String,
    pub current_animation_frame: usize,
    pub is_holstered: bool,
    pub flight_time: i32,
    pub collided_with_player: bool,
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
            initial_velocity: IntVector2D { x: 0, y: 0 },
            current_animation: "idle".to_string(),
            current_animation_frame: 0,
            is_holstered: true,
            flight_time: 0,
            collided_with_player: false,
        };
    }

    pub fn advance(
        &mut self,
        input: u8,
        prev_input: u8,
        player: &Player,
        other_player_hitbox: &Hitbox,
    ) {
        self.collided_with_player = false;
        if input_pressed(INPUT_ATTACK, input, prev_input) {
            let mut attack_heading = IntVector2D { x: 1, y: 0 };
            if player.is_facing_left {
                attack_heading.x = -1;
            }
            if input_check(INPUT_LEFT, input) {
                attack_heading.x = -1;
            } else if input_check(INPUT_RIGHT, input) {
                attack_heading.x = 1;
            } else if input_check(INPUT_UP, input)
                || input_check(INPUT_DOWN, input)
            {
                attack_heading.x = 0;
            }
            if input_check(INPUT_UP, input) {
                attack_heading.y = -1;
            } else if input_check(INPUT_DOWN, input) {
                attack_heading.y = 1;
            }
            self.velocity = attack_heading;
            self.velocity.normalize(MAX_SPEED);
            self.initial_velocity = self.velocity.clone();
            self.is_holstered = false;
        }
        if self.is_holstered {
            self.hitbox.x = player.center_x() - self.hitbox.width / 2;
            self.hitbox.y = player.center_y() - self.hitbox.height / 2;
            self.flight_time = 0;
        } else {
            let mut towards_player = IntVector2D {
                x: player.center_x() - self.center_x(),
                y: player.center_y() - self.center_y(),
            };
            let distance_from_player = towards_player.length_as_int();
            towards_player.normalize(MAX_SPEED);

            let mut lerp_factor = I32F32::from_num(self.flight_time)
                .saturating_div(I32F32::from_num(OG_FPS))
                .saturating_mul(RETURN_RATE);
            if lerp_factor > I32F32::ONE {
                lerp_factor = I32F32::ONE;
            }

            self.velocity.x = lerp(
                self.initial_velocity.x,
                towards_player.x,
                lerp_factor,
            );
            self.velocity.y = lerp(
                self.initial_velocity.y,
                towards_player.y,
                lerp_factor,
            );

            towards_player.normalize(MAX_SPEED / OG_FPS);

            if self.flight_time > 6
                && towards_player.length_as_int() >= distance_from_player
            {
                self.is_holstered = true;
                self.flight_time = 0;
            } else {
                self.move_by(
                    self.velocity.x / OG_FPS,
                    self.velocity.y / OG_FPS,
                    other_player_hitbox,
                );
                self.flight_time += 1;
            }
        }
        self.current_animation_frame += 1;
    }

    pub fn move_by(
        &mut self,
        move_x: i32,
        move_y: i32,
        other_player_hitbox: &Hitbox,
    ) {
        let mut sign = if move_x > 0 { 1 } else { -1 };
        let increments = [1000, 100, 10, 1];
        let mut increment_index = 0;
        let mut move_amount = move_x.abs();
        while increment_index < increments.len() {
            while move_amount >= increments[increment_index] {
                self.hitbox.x += increments[increment_index] * sign;
                self.check_entity_collisions(other_player_hitbox);
                move_amount -= increments[increment_index];
            }
            increment_index += 1;
        }

        sign = if move_y > 0 { 1 } else { -1 };
        increment_index = 0;
        move_amount = move_y.abs();
        while increment_index < increments.len() {
            while move_amount >= increments[increment_index] {
                self.hitbox.y += increments[increment_index] * sign;
                self.check_entity_collisions(other_player_hitbox);
                move_amount -= increments[increment_index];
            }
            increment_index += 1;
        }
    }

    pub fn check_entity_collisions(
        &mut self,
        other_player_hitbox: &Hitbox,
    ) {
        if do_hitboxes_overlap(&self.hitbox, other_player_hitbox) {
            self.collided_with_player = true;
        }
    }

    pub fn center_x(&self) -> i32 {
        return self.hitbox.x + self.hitbox.width / 2;
    }

    pub fn center_y(&self) -> i32 {
        return self.hitbox.y + self.hitbox.height / 2;
    }
}
