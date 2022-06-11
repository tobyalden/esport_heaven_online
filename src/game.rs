use serde::{Deserialize, Serialize};

pub struct Game {
    pub state: State,
}

impl Game {
    pub fn new() -> Self {
        Self {
            state: State::new(),
        }
    }
}

//game.register_local_handles(sess.local_player_handles());
//game.render();
//GGRSRequest::LoadGame { cell, .. } => self.load_game_state(cell),
//GGRSRequest::SaveGame { cell, frame } => self.save_game_state(cell, frame),
//GGRSRequest::AdvanceFrame { inputs } => self.advance_frame(inputs),

#[derive(Clone, Serialize, Deserialize)]
pub struct State {
    pub frame: i32,
    pub players: [Player; 2],
}

impl State {
    pub fn new() -> Self {
        let player_one = Player { x: 10, y: 20 };
        let player_two = Player { x: 50, y: 80 };
        Self {
            frame: 0,
            players: [player_one, player_two]
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Player {
    pub x: i32,
    pub y: i32,
}
