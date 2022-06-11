use tetra::graphics::{self, Color, DrawParams, Texture};
use tetra::math::Vec2;
use tetra::{Context, ContextBuilder, State};

//mod game;
//use hello_tetra_game::EsportGame;
mod game;
use game::Game;

struct Esport {
    game: Game,
    resources: Resources,
}

fn main() -> tetra::Result {
    ContextBuilder::new("Hello, world!", 320, 180)
        .quit_on_escape(true)
        .build()?
        .run(|ctx| {
            let game = Game::new();
            let resources = Resources::new(ctx);
            Ok(Esport { game, resources })
        })
}

impl State for Esport {
    fn draw(&mut self, ctx: &mut Context) -> tetra::Result {
        // Cornflower blue, as is tradition
        graphics::clear(ctx, Color::rgb(0.392, 0.584, 0.929));
        self.resources.player_one.draw(
            ctx, DrawParams::new().position(Vec2::new(self.game.state.players[0].x as f32, self.game.state.players[0].y as f32))
        );
        self.resources.player_two.draw(
            ctx, DrawParams::new().position(Vec2::new(self.game.state.players[1].x as f32, self.game.state.players[1].y as f32))
        );

        Ok(())
    }
}

struct Resources {
    player_one: Texture,
    player_two: Texture,
}

impl Resources {
    pub fn new(ctx: &mut Context) -> Self {
        let player_one = Texture::new(ctx, "./resources/player_one.png").unwrap();
        let player_two = Texture::new(ctx, "./resources/player_two.png").unwrap();
        Self {
            player_one,
            player_two,
        }
    }
}


//let mut game = Game::new(num_players);
//game.register_local_handles(sess.local_player_handles());
//game.render();
//GGRSRequest::LoadGame { cell, .. } => self.load_game_state(cell),
//GGRSRequest::SaveGame { cell, frame } => self.save_game_state(cell, frame),
//GGRSRequest::AdvanceFrame { inputs } => self.advance_frame(inputs),
