use ggrs::{
    GGRSError, P2PSession, PlayerType, SessionBuilder, SessionState,
    UdpNonBlockingSocket,
};
use instant::{Duration, Instant};
use std::collections::HashMap;
use std::net::SocketAddr;
use structopt::StructOpt;
use tetra::graphics::scaling::{ScalingMode, ScreenScaler};
use tetra::graphics::{self, Color, DrawParams, Rectangle, Texture};
use tetra::math::Vec2;
use tetra::time::Timestep;
use tetra::{Context, ContextBuilder, Event, State};

mod game;
mod level;
mod player;
mod utils;

use game::{GGRSConfig, Game};
use level::{Level, TILE_SIZE};
use player::Player;

const FPS: f64 = 60.0;

#[derive(StructOpt)]
struct Opt {
    #[structopt(short, long)]
    local_port: u16,
    #[structopt(short, long)]
    players: Vec<String>,
}

fn main() -> tetra::Result {
    // read cmd line arguments
    let opt = Opt::from_args();

    // create a GGRS session
    let mut sess_build = SessionBuilder::<GGRSConfig>::new()
        .with_num_players(2)
        .with_fps(FPS as usize)
        // (optional) set expected update frequency
        .unwrap()
        // (optional) set input delay for the local player
        .with_input_delay(1);

    // add players
    for (i, player_addr) in opt.players.iter().enumerate() {
        // local player
        if player_addr == "localhost" {
            sess_build =
                sess_build.add_player(PlayerType::Local, i).unwrap();
        } else {
            // remote players
            let remote_addr: SocketAddr = player_addr.parse().unwrap();
            sess_build = sess_build
                .add_player(PlayerType::Remote(remote_addr), i)
                .unwrap();
        }
    }

    // start the GGRS session
    let socket =
        UdpNonBlockingSocket::bind_to_port(opt.local_port).unwrap();
    let sess = sess_build.start_p2p_session(socket).unwrap();

    // time variables for tick rate
    let last_update = Instant::now();
    let accumulator = Duration::ZERO;

    ContextBuilder::new("esport heaven online", 640, 360)
        .quit_on_escape(true)
        .vsync(false)
        .resizable(true)
        .timestep(Timestep::Variable)
        .build()?
        .run(|ctx| {
            let mut game = Game::new();
            game.register_local_handles(sess.local_player_handles());

            let resources = Resources::new(ctx);
            let scaler = ScreenScaler::with_window_size(
                ctx,
                320,
                180,
                ScalingMode::ShowAll,
            )?;

            Ok(Esport {
                game,
                resources,
                sess,
                last_update,
                accumulator,
                scaler,
            })
        })
}

struct Esport {
    game: Game,
    resources: Resources,
    sess: P2PSession<GGRSConfig>,
    last_update: Instant,
    accumulator: Duration,
    scaler: ScreenScaler,
}

impl Esport {
    fn draw_player(
        &self,
        player: &Player,
        texture: &Texture,
        sprite: &Sprite,
        ctx: &mut Context,
    ) {
        let mut current_frame = player.current_animation_frame;
        current_frame = current_frame
            / sprite.animations[&player.current_animation].fps;
        current_frame = current_frame
            % sprite.animations[&player.current_animation].frames.len();
        texture.draw_region(
            ctx,
            Rectangle::new(
                sprite.animations[&player.current_animation].frames
                    [current_frame]
                    .x as f32,
                sprite.animations[&player.current_animation].frames
                    [current_frame]
                    .y as f32,
                sprite.frame_width as f32,
                sprite.frame_height as f32,
            ),
            DrawParams::new().position(Vec2::new(
                world_to_screen(player.hitbox.x),
                world_to_screen(player.hitbox.y),
            )),
        );
    }

    fn draw_tiles(
        &self,
        level: &Level,
        texture: &Texture,
        ctx: &mut Context,
    ) {
        for tile_x in 0..level.width_in_tiles {
            for tile_y in 0..level.height_in_tiles {
                if level.check_grid(tile_x, tile_y) {
                    texture.draw(
                        ctx,
                        DrawParams::new().position(Vec2::new(
                            world_to_screen(tile_x * TILE_SIZE),
                            world_to_screen(tile_y * TILE_SIZE),
                        )),
                    );
                }
            }
        }
    }
}

impl State for Esport {
    fn update(&mut self, ctx: &mut Context) -> tetra::Result {
        // communicate, receive and send packets
        self.sess.poll_remote_clients();

        // print GGRS events
        for event in self.sess.events() {
            println!("Event: {:?}", event);
        }

        // this is to keep ticks between clients synchronized.
        // if a client is ahead, it will run frames slightly slower
        // to allow catching up
        let mut fps_delta = 1. / FPS;
        if self.sess.frames_ahead() > 0 {
            fps_delta *= 1.1;
        }

        // get delta time from last iteration and accumulate it
        let delta = Instant::now().duration_since(self.last_update);
        self.accumulator = self.accumulator.saturating_add(delta);
        self.last_update = Instant::now();

        // if enough time is accumulated, we run a frame
        while self.accumulator.as_secs_f64() > fps_delta {
            // decrease self.accumulator
            self.accumulator = self
                .accumulator
                .saturating_sub(Duration::from_secs_f64(fps_delta));

            // frames are only happening if the self.sessions are
            // synchronized
            if self.sess.current_state() == SessionState::Running {
                // add input for all local players
                for handle in self.sess.local_player_handles() {
                    self.sess
                        .add_local_input(
                            handle,
                            self.game.local_input(ctx, handle),
                        )
                        .unwrap();
                }

                match self.sess.advance_frame() {
                    Ok(requests) => self.game.handle_requests(requests),
                    Err(GGRSError::PredictionThreshold) => {
                        println!(
                            "Frame {} skipped",
                            self.sess.current_frame()
                        )
                    }
                    Err(_) => {
                        println!("Unknown error")
                    }
                }
            }
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> tetra::Result {
        graphics::set_canvas(ctx, self.scaler.canvas());
        graphics::clear(ctx, Color::rgb(0.392, 0.584, 0.929));

        self.draw_player(
            &self.game.state.players[0],
            &self.resources.textures["player_one"],
            &self.resources.sprites["player_one"],
            ctx,
        );
        self.draw_player(
            &self.game.state.players[1],
            &self.resources.textures["player_two"],
            &self.resources.sprites["player_two"],
            ctx,
        );

        self.draw_tiles(
            &self.game.level,
            &self.resources.textures["tile"],
            ctx,
        );

        graphics::reset_canvas(ctx);
        graphics::clear(ctx, Color::BLACK);

        self.scaler.draw(ctx);

        Ok(())
    }

    fn event(&mut self, _: &mut Context, event: Event) -> tetra::Result {
        if let Event::Resized { width, height } = event {
            self.scaler.set_outer_size(width, height);
        }

        Ok(())
    }
}

fn world_to_screen(coordinate: i32) -> f32 {
    return coordinate as f32 / 1000.0;
}

#[derive(Clone)]
pub struct Sprite {
    frame_width: i32,
    frame_height: i32,
    animations: HashMap<String, Animation>,
}

#[derive(Clone)]
pub struct Animation {
    frames: Vec<Vec2<i32>>,
    fps: usize,
}

struct Resources {
    textures: HashMap<String, Texture>,
    sprites: HashMap<String, Sprite>,
}

impl Resources {
    pub fn new(ctx: &mut Context) -> Self {
        let textures = HashMap::from([
            (
                "player_one".to_string(),
                Texture::new(ctx, "./resources/graphics/player_one.png")
                    .unwrap(),
            ),
            (
                "player_two".to_string(),
                Texture::new(ctx, "./resources/graphics/player_two.png")
                    .unwrap(),
            ),
            (
                "tile".to_string(),
                Texture::new(ctx, "./resources/graphics/tile.png")
                    .unwrap(),
            ),
        ]);
        // TODO: Create helper functions to simplify this process
        let mut sprites = HashMap::from([(
            "player_one".to_string(),
            Sprite {
                frame_width: 8,
                frame_height: 12,
                animations: HashMap::new(),
            },
        )]);
        let frame_width = sprites["player_one"].frame_width;
        let frame_height = sprites["player_one"].frame_height;
        sprites.get_mut("player_one").unwrap().animations.insert(
            "idle".to_string(),
            Animation {
                frames: get_frames(
                    &textures["player_one"],
                    frame_width,
                    frame_height,
                    &[0],
                ),
                fps: 1,
            },
        );
        sprites.get_mut("player_one").unwrap().animations.insert(
            "run".to_string(),
            Animation {
                frames: get_frames(
                    &textures["player_one"],
                    frame_width,
                    frame_height,
                    &[1, 2, 3, 2],
                ),
                fps: 60,
            },
        );
        sprites.insert(
            "player_two".to_string(),
            sprites["player_one"].clone(),
        );
        Self { textures, sprites }
    }
}

fn get_frames(
    texture: &Texture,
    frame_width: i32,
    frame_height: i32,
    frame_indices: &[i32],
) -> Vec<Vec2<i32>> {
    let num_columns = texture.width() / frame_width;
    let mut frames: Vec<Vec2<i32>> = Vec::new();
    for frame_index in frame_indices {
        let frame_x = (frame_index % num_columns) * frame_width;
        let frame_y = (frame_index / num_columns) * frame_height;
        let frame = Vec2 {
            x: frame_x,
            y: frame_y,
        };
        frames.push(frame);
    }
    return frames;
}

//sprite = new Spritemap('graphics/player${playerNumber}.png', 8, 12);
//sprite.flipX = playerNumber == 2;
//sprite.add("idle", [0]);
//sprite.add("run", [1, 2, 3, 2], 8);
//sprite.add("jump", [4]);
//sprite.add("wall", [5]);
//sprite.add("skid", [6]);
//sprite.add("slide", [7]);
