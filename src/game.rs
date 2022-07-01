use bytemuck::{Pod, Zeroable};
use ggrs::{
    Config, Frame, GGRSRequest, GameStateCell, InputStatus, PlayerHandle,
    NULL_FRAME,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tetra::input::{self, GamepadAxis, GamepadButton, Key};
use tetra::Context;

use crate::boomerang::Boomerang;
use crate::level::Level;
use crate::player::Player;

const CHECKSUM_PERIOD: i32 = 100;

pub const INPUT_UP: u8 = 1 << 0;
pub const INPUT_DOWN: u8 = 1 << 1;
pub const INPUT_LEFT: u8 = 1 << 2;
pub const INPUT_RIGHT: u8 = 1 << 3;
pub const INPUT_JUMP: u8 = 1 << 4;
pub const INPUT_ATTACK: u8 = 1 << 5;
pub const INPUT_DODGE: u8 = 1 << 6;

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Pod, Zeroable)]
pub struct Input {
    pub inp: u8,
}

// GGRSConfig holds all type parameters for GGRS Sessions
#[derive(Debug)]
pub struct GGRSConfig;
impl Config for GGRSConfig {
    type Input = Input;
    type State = State;
    type Address = SocketAddr;
}

// computes the fletcher16 checksum, copied from wikipedia:
// <https://en.wikipedia.org/wiki/Fletcher%27s_checksum>
fn fletcher16(data: &[u8]) -> u16 {
    let mut sum1: u16 = 0;
    let mut sum2: u16 = 0;
    for index in 0..data.len() {
        sum1 = (sum1 + data[index] as u16) % 255;
        sum2 = (sum2 + sum1) % 255;
    }
    (sum2 << 8) | sum1
}

pub struct Game {
    pub state: State,
    pub level: Level,
    local_handles: Vec<PlayerHandle>,
    last_checksum: (Frame, u64),
    periodic_checksum: (Frame, u64),
}

impl Game {
    pub fn new() -> Self {
        Self {
            state: State::new(),
            level: Level::new(),
            local_handles: Vec::new(),
            last_checksum: (NULL_FRAME, 0),
            periodic_checksum: (NULL_FRAME, 0),
        }
    }

    // for each request, call the appropriate function
    pub fn handle_requests(
        &mut self,
        requests: Vec<GGRSRequest<GGRSConfig>>,
    ) {
        for request in requests {
            match request {
                GGRSRequest::LoadGameState { cell, .. } => {
                    self.load_game_state(cell)
                }
                GGRSRequest::SaveGameState { cell, frame } => {
                    self.save_game_state(cell, frame)
                }
                GGRSRequest::AdvanceFrame { inputs } => {
                    self.advance_frame(inputs)
                }
            }
        }
    }

    pub fn advance_frame(&mut self, inputs: Vec<(Input, InputStatus)>) {
        self.state.advance(inputs, &self.level);

        if self.state.round_end_frame != -1 && self.state.frame - self.state.round_end_frame > 60 * 5 {
            self.state.reset();
        }

        // remember checksum to render it later
        // it is very inefficient to serialize the gamestate here
        // just for the checksum
        let buffer = bincode::serialize(&self.state).unwrap();
        let checksum = fletcher16(&buffer) as u64;
        self.last_checksum = (self.state.frame, checksum);
        if self.state.frame % CHECKSUM_PERIOD == 0 {
            self.periodic_checksum = (self.state.frame, checksum);
        }
    }

    // save current gamestate, create a checksum
    // creating a checksum here is only relevant for SyncTestSessions
    fn save_game_state(
        &mut self,
        cell: GameStateCell<State>,
        frame: Frame,
    ) {
        assert_eq!(self.state.frame, frame);
        let buffer = bincode::serialize(&self.state).unwrap();
        let checksum = fletcher16(&buffer) as u128;
        cell.save(frame, Some(self.state.clone()), Some(checksum));
    }

    // load gamestate and overwrite
    fn load_game_state(&mut self, cell: GameStateCell<State>) {
        self.state = cell.load().expect("No data found.");
    }

    pub fn register_local_handles(&mut self, handles: Vec<PlayerHandle>) {
        self.local_handles = handles
    }

    pub fn local_input(
        &self,
        ctx: &mut Context,
        handle: PlayerHandle,
    ) -> Input {
        let mut inp: u8 = 0;
        if handle == self.local_handles[0] {
            // first local player with WASD or controller
            if input::is_key_down(ctx, Key::W)
                || input::get_gamepad_axis_position(
                    ctx,
                    0,
                    GamepadAxis::LeftStickY,
                ) < -0.5
            {
                inp |= INPUT_UP;
            }
            if input::is_key_down(ctx, Key::A)
                || input::get_gamepad_axis_position(
                    ctx,
                    0,
                    GamepadAxis::LeftStickX,
                ) < -0.5
            {
                inp |= INPUT_LEFT;
            }
            if input::is_key_down(ctx, Key::S)
                || input::get_gamepad_axis_position(
                    ctx,
                    0,
                    GamepadAxis::LeftStickY,
                ) > 0.5
            {
                inp |= INPUT_DOWN;
            }
            if input::is_key_down(ctx, Key::D)
                || input::get_gamepad_axis_position(
                    ctx,
                    0,
                    GamepadAxis::LeftStickX,
                ) > 0.5
            {
                inp |= INPUT_RIGHT;
            }
            if input::is_key_down(ctx, Key::J)
                || input::is_gamepad_button_down(ctx, 0, GamepadButton::A)
            {
                inp |= INPUT_JUMP;
            }
            if input::is_key_down(ctx, Key::K)
                || input::is_gamepad_button_down(ctx, 0, GamepadButton::X)
            {
                inp |= INPUT_ATTACK;
            }
            if input::is_key_down(ctx, Key::L)
                || input::get_gamepad_axis_position(
                    ctx,
                    0,
                    GamepadAxis::RightTrigger,
                ) > 0.5
            {
                inp |= INPUT_DODGE;
            }
        } else {
            // all other local players with arrow keys
            if input::is_key_down(ctx, Key::Up) {
                inp |= INPUT_UP;
            }
            if input::is_key_down(ctx, Key::Left) {
                inp |= INPUT_LEFT;
            }
            if input::is_key_down(ctx, Key::Down) {
                inp |= INPUT_DOWN;
            }
            if input::is_key_down(ctx, Key::Right) {
                inp |= INPUT_RIGHT;
            }
            if input::is_key_down(ctx, Key::Z) {
                inp |= INPUT_JUMP;
            }
            if input::is_key_down(ctx, Key::X) {
                inp |= INPUT_ATTACK;
            }
            if input::is_key_down(ctx, Key::C) {
                inp |= INPUT_DODGE;
            }
        }
        Input { inp }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct State {
    pub frame: i32,
    pub prev_inputs: [u8; 2],
    pub players: [Player; 2],
    pub boomerangs: [Boomerang; 2],
    pub round_end_frame: i32,
}

impl State {
    pub fn new() -> Self {
        let player_one = Player::new(50000, 80000, false);
        let player_two = Player::new(200000, 80000, true);
        Self {
            frame: 0,
            prev_inputs: [0, 0],
            players: [player_one, player_two],
            boomerangs: [Boomerang::new(), Boomerang::new()],
            round_end_frame: -1
        }
    }

    pub fn reset(&mut self) {
        let player_one = Player::new(50000, 80000, false);
        let player_two = Player::new(200000, 80000, true);
        self.prev_inputs = [0, 0];
        self.players = [player_one, player_two];
        self.boomerangs = [Boomerang::new(), Boomerang::new()];
        self.round_end_frame = -1;
    }

    pub fn advance(
        &mut self,
        inputs: Vec<(Input, InputStatus)>,
        level: &Level,
    ) {
        self.frame += 1;

        // update players
        for player_num in 0..2 {
            if self.players[player_num].is_dead {
                continue;
            }
            let input = inputs[player_num].0.inp;
            let other_player_hitbox =
                &self.players[1 - player_num].hitbox.clone();
            let other_boomerang_hitbox =
                &self.boomerangs[1 - player_num].hitbox.clone();
            self.players[player_num].advance(
                input,
                self.prev_inputs[player_num],
                level,
                other_player_hitbox,
                other_boomerang_hitbox,
            );
        }

        // update boomerangs
        for player_num in 0..2 {
            if self.players[player_num].is_dead {
                continue;
            }
            let input = inputs[player_num].0.inp;
            let other_player_hitbox =
                &self.players[1 - player_num].hitbox.clone();
            self.boomerangs[player_num].advance(
                input,
                self.prev_inputs[player_num],
                &self.players[player_num],
                other_player_hitbox,
            );
        }

        // combat interactions
        for player_num in 0..2 {
            if self.boomerangs[player_num].collided_with_player {
                self.players[1 - player_num].collided_with_boomerang =
                    true;
            }
        }
        for player_num in 0..2 {
            if self.players[player_num].collided_with_player
                && self.players[player_num].dodge_timer == 0
                && self.players[1 - player_num].dodge_timer > 0
            {
                self.players[player_num].will_die = true;
            }

            if self.players[player_num].collided_with_boomerang
                && self.players[player_num].dodge_timer == 0
                && !self.boomerangs[1 - player_num].is_holstered
            {
                self.players[player_num].will_die = true;
            }
        }

        // kill players
        for player_num in 0..2 {
            if self.players[player_num].will_die {
                self.players[player_num].will_die = false;
                self.players[player_num].is_dead = true;
                self.boomerangs[player_num].is_holstered = true;
                self.round_end_frame = self.frame;
            }
        }

        // set previous inputs
        for player_num in 0..2 {
            let input = inputs[player_num].0.inp;
            self.prev_inputs[player_num] = input;
        }
    }
}
