use serde::{Deserialize, Serialize};

pub const MAX_OPACITY: i32 = 100;

#[derive(Clone, Serialize, Deserialize)]
pub struct Curtain {
    pub opacity: i32,
}

impl Curtain {
    pub fn new() -> Curtain {
        return Curtain {
            opacity: MAX_OPACITY,
        };
    }

    pub fn advance(&mut self) {
        self.opacity -= 1;
        if self.opacity < 0 {
            self.opacity = 0;
        }
    }
}
