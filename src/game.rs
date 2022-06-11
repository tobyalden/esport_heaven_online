use bytemuck::{Pod, Zeroable};
use ggrs::{InputStatus};
use serde::{Deserialize, Serialize};
use tetra::{Context};
use tetra::input::{self, Key};

const INPUT_UP: u8 = 1 << 0;
const INPUT_DOWN: u8 = 1 << 1;
const INPUT_LEFT: u8 = 1 << 2;
const INPUT_RIGHT: u8 = 1 << 3;

pub struct Game {
    pub state: State,
}

impl Game {
    pub fn new() -> Self {
        Self {
            state: State::new(),
        }
    }

    pub fn advance_frame(&mut self, inputs: Vec<(Input, InputStatus)>) {
        self.state.advance(inputs);
    }

    pub fn local_input(&self, ctx: &mut Context) -> Input {
        let mut inp: u8 = 0;
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
        Input { inp }
    }
}

//game.register_local_handles(sess.local_player_handles());
//game.render();
//GGRSRequest::LoadGame { cell, .. } => self.load_game_state(cell),
//GGRSRequest::SaveGame { cell, frame } => self.save_game_state(cell, frame),
//GGRSRequest::AdvanceFrame { inputs } => self.advance_frame(inputs),

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Pod, Zeroable)]
pub struct Input {
    pub inp: u8,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct State {
    pub frame: i32,
    pub players: [Player; 2],
}

impl State {
    pub fn new() -> Self {
        let player_one = Player { x: 50, y: 80 };
        let player_two = Player { x: 200, y: 80 };
        Self {
            frame: 0,
            players: [player_one, player_two]
        }
    }

    pub fn advance(&mut self, inputs: Vec<(Input, InputStatus)>) {
        self.frame += 1;

        for player_num in 0..2 {
            let input = inputs[player_num].0.inp;
            if input & INPUT_UP != 0 && input & INPUT_DOWN == 0 {
                self.players[player_num].y -= 2;
            }
            if input & INPUT_UP == 0 && input & INPUT_DOWN != 0 {
                self.players[player_num].y += 2;
            }
            if input & INPUT_LEFT != 0 && input & INPUT_RIGHT == 0 {
                self.players[player_num].x -= 2;
            }
            if input & INPUT_LEFT == 0 && input & INPUT_RIGHT != 0 {
                self.players[player_num].x += 2;
            }
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Player {
    pub x: i32,
    pub y: i32,
}
