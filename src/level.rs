use quick_xml::de::from_str;
use serde::Deserialize;
use std::fs;

pub const TILE_SIZE: i32 = 4000;

pub struct Level {
    pub width_in_tiles: i32,
    pub height_in_tiles: i32,
    pub grid: Vec<bool>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct LevelData {
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
