use bytemuck::{Pod, Zeroable};
use ggrs::{
    Config, Frame, GGRSRequest, GameStateCell, InputStatus, PlayerHandle,
    NULL_FRAME,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tetra::input::{self, Key};
use tetra::Context;

use crate::level::Level;
use crate::player::Player;
//use crate::player;

const CHECKSUM_PERIOD: i32 = 100;

pub const INPUT_UP: u8 = 1 << 0;
pub const INPUT_DOWN: u8 = 1 << 1;
pub const INPUT_LEFT: u8 = 1 << 2;
pub const INPUT_RIGHT: u8 = 1 << 3;

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
            // first local player with WASD
            if input::is_key_down(ctx, Key::W) {
                inp |= INPUT_UP;
            }
            if input::is_key_down(ctx, Key::A) {
                inp |= INPUT_LEFT;
            }
            if input::is_key_down(ctx, Key::S) {
                inp |= INPUT_DOWN;
            }
            if input::is_key_down(ctx, Key::D) {
                inp |= INPUT_RIGHT;
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
        }
        Input { inp }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct State {
    pub frame: i32,
    pub players: [Player; 2],
}

impl State {
    pub fn new() -> Self {
        let player_one = Player::new(50000, 80000, false);
        let player_two = Player::new(200000, 80000, true);
        Self {
            frame: 0,
            players: [player_one, player_two],
        }
    }

    pub fn advance(
        &mut self,
        inputs: Vec<(Input, InputStatus)>,
        level: &Level,
    ) {
        self.frame += 1;

        for player_num in 0..2 {
            let input = inputs[player_num].0.inp;
            self.players[player_num].advance(input, level);
        }
    }
}
