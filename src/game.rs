use bytemuck::{Pod, Zeroable};
use ggrs::{Config, Frame, GGRSRequest, GameStateCell, InputStatus, PlayerHandle, NULL_FRAME};
use quick_xml::de::{from_str};
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::SocketAddr;
use tetra::{Context};
use tetra::input::{self, Key};

const CHECKSUM_PERIOD: i32 = 100;

const INPUT_UP: u8 = 1 << 0;
const INPUT_DOWN: u8 = 1 << 1;
const INPUT_LEFT: u8 = 1 << 2;
const INPUT_RIGHT: u8 = 1 << 3;

pub const TILE_SIZE: i32 = 4;

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

// computes the fletcher16 checksum, copied from wikipedia: <https://en.wikipedia.org/wiki/Fletcher%27s_checksum>
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
    pub fn handle_requests(&mut self, requests: Vec<GGRSRequest<GGRSConfig>>) {
        for request in requests {
            match request {
                GGRSRequest::LoadGameState { cell, .. } => self.load_game_state(cell),
                GGRSRequest::SaveGameState { cell, frame } => self.save_game_state(cell, frame),
                GGRSRequest::AdvanceFrame { inputs } => self.advance_frame(inputs),
            }
        }
    }

    pub fn advance_frame(&mut self, inputs: Vec<(Input, InputStatus)>) {
        //println!("Advancing frame");
        self.state.advance(inputs);

        // remember checksum to render it later
        // it is very inefficient to serialize the gamestate here just for the checksum
        let buffer = bincode::serialize(&self.state).unwrap();
        let checksum = fletcher16(&buffer) as u64;
        self.last_checksum = (self.state.frame, checksum);
        if self.state.frame % CHECKSUM_PERIOD == 0 {
            self.periodic_checksum = (self.state.frame, checksum);
        }
    }

    // save current gamestate, create a checksum
    // creating a checksum here is only relevant for SyncTestSessions
    fn save_game_state(&mut self, cell: GameStateCell<State>, frame: Frame) {
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

    pub fn local_input(&self, ctx: &mut Context, handle: PlayerHandle) -> Input {
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
        }
        else {
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
        let player_one = Player { x: 50000, y: 80000 };
        let player_two = Player { x: 200000, y: 80000 };
        Self {
            frame: 0,
            players: [player_one, player_two]
        }
    }

    pub fn advance(&mut self, inputs: Vec<(Input, InputStatus)>) {
        self.frame += 1;

        for player_num in 0..2 {
            let input = inputs[player_num].0.inp;
            //println!("Player {} input: {}", player_num, input);
            if input & INPUT_UP != 0 && input & INPUT_DOWN == 0 {
                self.players[player_num].y -= 2000;
            }
            if input & INPUT_UP == 0 && input & INPUT_DOWN != 0 {
                self.players[player_num].y += 2000;
            }
            if input & INPUT_LEFT != 0 && input & INPUT_RIGHT == 0 {
                self.players[player_num].x -= 2000;
            }
            if input & INPUT_LEFT == 0 && input & INPUT_RIGHT != 0 {
                self.players[player_num].x += 2000;
            }
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Player {
    pub x: i32,
    pub y: i32,
}

pub struct Level {
    pub width_in_tiles: i32,
    pub height_in_tiles: i32,
    pub grid: Vec<bool>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct LevelData {
    width: i32,
    height: i32,
    solids: String,
}

impl Level {
    pub fn new() -> Self {
        let xml = fs::read_to_string("./resources/levels/level.oel").unwrap();
        let data:LevelData = from_str(&xml).unwrap();
        let width_in_tiles: i32 = data.width / TILE_SIZE;
        let height_in_tiles: i32 = data.height / TILE_SIZE;
        let mut grid = Vec::new();
        for c in data.solids.chars() {
            if c == '\n' {
                continue;
            }
            grid.push(c == '1');
         }
        //println!("width_in_tiles: {}", width_in_tiles);
        //println!("height_in_tiles: {}", height_in_tiles);
        //println!("grid: {:?}", grid);
        //println!("grid length: {}", grid.len());
        Self { width_in_tiles, height_in_tiles, grid }
    }
}
