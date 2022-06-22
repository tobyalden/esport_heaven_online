use bytemuck::{Pod, Zeroable};
use ggrs::{
    Config, Frame, GGRSRequest, GameStateCell, InputStatus, PlayerHandle,
    NULL_FRAME,
};
use quick_xml::de::from_str;
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::SocketAddr;
use tetra::input::{self, Key};
use tetra::Context;

const CHECKSUM_PERIOD: i32 = 100;

const INPUT_UP: u8 = 1 << 0;
const INPUT_DOWN: u8 = 1 << 1;
const INPUT_LEFT: u8 = 1 << 2;
const INPUT_RIGHT: u8 = 1 << 3;

pub const TILE_SIZE: i32 = 4000;

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
        let player_one = Player {
            hitbox: Hitbox {
                x: 50000,
                y: 80000,
                width: 17000,
                height: 17000,
            },
            velocity: IntVector2D { x: 0, y: 0 },
        };
        let player_two = Player {
            hitbox: Hitbox {
                x: 200000,
                y: 80000,
                width: 17000,
                height: 17000,
            },
            velocity: IntVector2D { x: 0, y: 0 },
        };
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
            self.players[player_num].velocity.zero();
            if input & INPUT_UP != 0 && input & INPUT_DOWN == 0 {
                self.players[player_num].velocity.y = -800;
            }
            if input & INPUT_UP == 0 && input & INPUT_DOWN != 0 {
                self.players[player_num].velocity.y = 800;
            }
            if input & INPUT_LEFT != 0 && input & INPUT_RIGHT == 0 {
                self.players[player_num].velocity.x = -800;
            }
            if input & INPUT_LEFT == 0 && input & INPUT_RIGHT != 0 {
                self.players[player_num].velocity.x = 800;
            }
            self.players[player_num].move_by(
                level,
                self.players[player_num].velocity.x,
                self.players[player_num].velocity.y,
            );
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Player {
    pub hitbox: Hitbox,
    pub velocity: IntVector2D,
}

impl Player {
    fn move_by(&mut self, level: &Level, move_x: i32, move_y: i32) {
        if collide(self, level, self.hitbox.x + move_x, self.hitbox.y) {
            let sign = if move_x > 0 { 1 } else { -1 };
            while !collide(
                self,
                level,
                self.hitbox.x + sign,
                self.hitbox.y,
            ) {
                self.hitbox.x += sign;
            }
        } else {
            self.hitbox.x += move_x;
        }
        if collide(self, level, self.hitbox.x, self.hitbox.y + move_y) {
            let sign = if move_y > 0 { 1 } else { -1 };
            while !collide(
                self,
                level,
                self.hitbox.x,
                self.hitbox.y + sign,
            ) {
                self.hitbox.y += sign;
            }
        } else {
            self.hitbox.y += move_y;
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntVector2D {
    pub x: i32,
    pub y: i32,
}

impl IntVector2D {
    pub fn zero(&mut self) {
        self.x = 0;
        self.y = 0;
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Hitbox {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

fn collide(
    player: &Player,
    level: &Level,
    virtual_x: i32,
    virtual_y: i32,
) -> bool {
    let player_hitbox = Hitbox {
        x: virtual_x,
        y: virtual_y,
        width: player.hitbox.width,
        height: player.hitbox.height,
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

fn do_hitboxes_overlap(a: &Hitbox, b: &Hitbox) -> bool {
    let is_not_overlapping = a.x > b.x + b.width
        || b.x > a.x + a.width
        || a.y > b.y + b.height
        || b.y > a.y + a.height;
    return !is_not_overlapping;
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
        let xml =
            fs::read_to_string("./resources/levels/level.oel").unwrap();
        let data: LevelData = from_str(&xml).unwrap();
        let width_in_tiles: i32 = data.width / 4;
        let height_in_tiles: i32 = data.height / 4;
        let mut grid = Vec::new();
        for c in data.solids.chars() {
            if c == '\n' {
                continue;
            }
            grid.push(c == '1');
        }
        Self {
            width_in_tiles,
            height_in_tiles,
            grid,
        }
    }

    pub fn check_grid(&self, tile_x: i32, tile_y: i32) -> bool {
        if tile_x < 0
            || tile_x >= self.width_in_tiles
            || tile_y < 0
            || tile_y >= self.height_in_tiles
        {
            return false;
        }
        return self.grid
            [(tile_x + tile_y * self.width_in_tiles) as usize];
    }
}
