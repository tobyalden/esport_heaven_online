use ggrs::{
    GGRSError, P2PSession, PlayerType, SessionBuilder, SessionState,
    UdpNonBlockingSocket,
};
use instant::{Duration, Instant};
use std::collections::HashMap;
use std::net::SocketAddr;
use structopt::StructOpt;
use tetra::audio::{Sound, SoundInstance};
//use tetra::graphics::mesh::{Mesh, ShapeStyle};
use tetra::graphics::scaling::{ScalingMode, ScreenScaler};
use tetra::graphics::{self, Color, DrawParams, Rectangle, Texture};
use tetra::math::Vec2;
use tetra::time::Timestep;
use tetra::{Context, ContextBuilder, Event, State};

mod boomerang;
mod game;
mod level;
mod particle;
mod player;
mod utils;

use boomerang::Boomerang;
use game::{GGRSConfig, Game};
use level::{Level, TILE_SIZE};
use particle::{
    Particle, GROUND_DUST_ANIMATION_FRAMES, GROUND_DUST_ANIMATION_SPEED,
};
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
        //let simple = Mesh::rectangle(
        //ctx,
        //ShapeStyle::Fill,
        //Rectangle {
        //x: 0.0,
        //y: 0.0,
        //width: 6.0,
        //height: 12.0,
        //},
        //)
        //.unwrap();
        //simple.draw(
        //ctx,
        //Vec2::new(
        //world_to_screen(player.hitbox.x),
        //world_to_screen(player.hitbox.y),
        //),
        //);

        if player.is_dead {
            return;
        }

        let mut current_frame = player.current_animation_frame;
        current_frame = current_frame
            / sprite.animations[&player.current_animation].fps;
        current_frame = current_frame
            % sprite.animations[&player.current_animation].frames.len();
        let scale_x = if player.is_facing_left { -1.0 } else { 1.0 };
        let color = if player.dodge_timer > 0 {
            Color::BLACK
        } else {
            Color::WHITE
        };
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
            DrawParams::new()
                .position(Vec2::new(
                    world_to_screen(
                        player.hitbox.x + player.hitbox.width / 2,
                    ),
                    world_to_screen(
                        player.hitbox.y + player.hitbox.height / 2,
                    ),
                ))
                .origin(Vec2::new(
                    sprite.frame_width as f32 / 2.0,
                    sprite.frame_height as f32 / 2.0,
                ))
                .scale(Vec2::new(scale_x, 1.0))
                .color(color),
        );
    }

    fn draw_boomerang(
        &self,
        boomerang: &Boomerang,
        texture: &Texture,
        sprite: &Sprite,
        ctx: &mut Context,
    ) {
        if boomerang.is_holstered {
            return;
        }
        let mut current_frame = boomerang.current_animation_frame;
        current_frame = current_frame
            / sprite.animations[&boomerang.current_animation].fps;
        current_frame = current_frame
            % sprite.animations[&boomerang.current_animation].frames.len();
        texture.draw_region(
            ctx,
            Rectangle::new(
                sprite.animations[&boomerang.current_animation].frames
                    [current_frame]
                    .x as f32,
                sprite.animations[&boomerang.current_animation].frames
                    [current_frame]
                    .y as f32,
                sprite.frame_width as f32,
                sprite.frame_height as f32,
            ),
            DrawParams::new()
                .position(Vec2::new(
                    world_to_screen(
                        boomerang.hitbox.x + boomerang.hitbox.width / 2,
                    ),
                    world_to_screen(
                        boomerang.hitbox.y + boomerang.hitbox.height / 2,
                    ),
                ))
                .origin(Vec2::new(
                    sprite.frame_width as f32 / 2.0,
                    sprite.frame_height as f32 / 2.0,
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

    fn draw_particle(
        &self,
        particle: &Particle,
        texture: &Texture,
        sprite: &Sprite,
        ctx: &mut Context,
    ) {
        if particle.current_animation == "none" {
            return;
        }
        let mut current_frame = particle.current_animation_frame;
        current_frame = current_frame
            / sprite.animations[&particle.current_animation].fps;
        current_frame = current_frame
            % sprite.animations[&particle.current_animation].frames.len();
        texture.draw_region(
            ctx,
            Rectangle::new(
                sprite.animations[&particle.current_animation].frames
                    [current_frame]
                    .x as f32,
                sprite.animations[&particle.current_animation].frames
                    [current_frame]
                    .y as f32,
                sprite.frame_width as f32,
                sprite.frame_height as f32,
            ),
            DrawParams::new()
                .position(Vec2::new(
                    world_to_screen(particle.position.x),
                    world_to_screen(particle.position.y),
                ))
                .origin(Vec2::new(
                    sprite.frame_width as f32 / 2.0,
                    sprite.frame_height as f32 / 2.0,
                )),
        );
    }

    fn handle_sounds(&mut self) {
        for player_num in 0..2 {
            for _ in
                0..self.game.state.players[player_num].sound_commands.len()
            {
                let sound_command = self.game.state.players[player_num]
                    .sound_commands
                    .pop()
                    .unwrap();
                self.handle_sound_command(player_num, &sound_command);
            }
            for _ in 0..self.game.state.boomerangs[player_num]
                .sound_commands
                .len()
            {
                let sound_command = self.game.state.boomerangs[player_num]
                    .sound_commands
                    .pop()
                    .unwrap();
                self.handle_sound_command(player_num, &sound_command);
            }
        }
    }

    fn handle_sound_command(
        &mut self,
        player_num: usize,
        sound_command: &(String, String, i32),
    ) {
        let sound = &self.resources.sounds
            [&get_player_sound_name(player_num, &sound_command.0)];
        let volume: f32 = sound_command.2 as f32 / 100.0;
        if sound_command.1 == "play".to_string() {
            sound.stop();
            sound.set_volume(volume);
            sound.play();
        } else if sound_command.1 == "loop".to_string() {
            sound.set_repeating(true);
            sound.set_volume(volume);
            sound.play();
        } else if sound_command.1 == "stop".to_string() {
            sound.stop();
        }
    }
}

fn get_player_sound_name(player_num: usize, sound_name: &str) -> String {
    return format!("player{}-{}", player_num, sound_name);
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

                self.handle_sounds()
            }
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> tetra::Result {
        graphics::set_canvas(ctx, self.scaler.canvas());
        graphics::clear(ctx, Color::rgb(0.392, 0.584, 0.929));

        self.draw_tiles(
            &self.game.level,
            &self.resources.textures["tile"],
            ctx,
        );

        self.draw_boomerang(
            &self.game.state.boomerangs[0],
            &self.resources.textures["boomerang_one"],
            &self.resources.sprites["boomerang_one"],
            ctx,
        );
        self.draw_boomerang(
            &self.game.state.boomerangs[1],
            &self.resources.textures["boomerang_two"],
            &self.resources.sprites["boomerang_two"],
            ctx,
        );

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

        for particle in &self.game.state.particles {
            self.draw_particle(
                particle,
                &self.resources.textures["particle"],
                &self.resources.sprites["particle"],
                ctx,
            );
        }

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
    texture_width: i32,
    frame_width: i32,
    frame_height: i32,
    animations: HashMap<String, Animation>,
}

impl Sprite {
    fn new(
        texture_width: i32,
        frame_width: i32,
        frame_height: i32,
    ) -> Sprite {
        return Sprite {
            texture_width,
            frame_width,
            frame_height,
            animations: HashMap::new(),
        };
    }

    fn add(&mut self, name: String, frame_indices: &[i32], fps: usize) {
        self.animations.insert(
            name,
            Animation {
                frames: get_frames(
                    self.texture_width,
                    self.frame_width,
                    self.frame_height,
                    frame_indices,
                ),
                fps,
            },
        );
    }
}

#[derive(Clone)]
pub struct Animation {
    frames: Vec<Vec2<i32>>,
    fps: usize,
}

struct Resources {
    textures: HashMap<String, Texture>,
    sprites: HashMap<String, Sprite>,
    sounds: HashMap<String, SoundInstance>,
}

impl Resources {
    pub fn new(ctx: &mut Context) -> Self {
        let mut textures: HashMap<String, Texture> = HashMap::new();
        for name in [
            "player_one",
            "player_two",
            "tile",
            "boomerang_one",
            "boomerang_two",
            "particle",
        ] {
            textures.insert(
                name.to_string(),
                Texture::new(
                    ctx,
                    format!("./resources/graphics/{}.png", name),
                )
                .unwrap(),
            );
        }

        let mut player_one_sprite =
            Sprite::new(textures["player_one"].width(), 8, 12);
        let mut player_two_sprite =
            Sprite::new(textures["player_two"].width(), 8, 12);
        for sprite in [&mut player_one_sprite, &mut player_two_sprite] {
            sprite.add("idle".to_string(), &[0], 1);
            sprite.add("run".to_string(), &[1, 2, 3, 2], 8);
            sprite.add("jump".to_string(), &[4], 1);
            sprite.add("wall".to_string(), &[5], 1);
            sprite.add("skid".to_string(), &[6], 1);
            sprite.add("slide".to_string(), &[7], 1);
        }

        let mut boomerang_one_sprite =
            Sprite::new(textures["boomerang_one"].width(), 8, 8);
        let mut boomerang_two_sprite =
            Sprite::new(textures["boomerang_two"].width(), 8, 8);
        for sprite in
            [&mut boomerang_one_sprite, &mut boomerang_two_sprite]
        {
            sprite.add("idle".to_string(), &[0], 1);
        }

        let mut particle_sprite =
            Sprite::new(textures["particle"].width(), 8, 4);

        // We do this to avoid hardcoding the number of animation frames twice (here in main.rs and in particle.rs)
        let mut ground_dust_frames = [0; GROUND_DUST_ANIMATION_FRAMES];
        for (i, v) in ground_dust_frames.iter_mut().enumerate() {
            *v = i as i32
        }
        particle_sprite.add(
            "grounddust".to_string(),
            &ground_dust_frames,
            GROUND_DUST_ANIMATION_SPEED,
        );

        let sprites = HashMap::from([
            ("player_one".to_string(), player_one_sprite),
            ("player_two".to_string(), player_two_sprite),
            ("boomerang_one".to_string(), boomerang_one_sprite),
            ("boomerang_two".to_string(), boomerang_two_sprite),
            ("particle".to_string(), particle_sprite),
        ]);

        let mut sounds: HashMap<String, SoundInstance> = HashMap::new();
        for name in [
            //"addfinalpoint",
            //"addpoint",
            "catch",
            "death",
            "dodge",
            //"dodge1",
            //"dodge2",
            //"dodge3",
            //"dodge4",
            "doublejump",
            //"fight",
            //"gameover",
            "jump",
            "land",
            //"menuselect",
            //"menustart",
            //"ready",
            "run",
            //"showscoreboard",
            "skid",
            "superjump",
            "toss",
            "wallslide",
            "whoosh",
        ] {
            for player_num in 0..2 {
                sounds.insert(
                    format!("player{}-{}", player_num, name),
                    Sound::new(format!("./resources/audio/{}.wav", name))
                        .unwrap()
                        .spawn(ctx)
                        .unwrap(),
                );
            }
        }

        Self {
            textures,
            sprites,
            sounds,
        }
    }
}

fn get_frames(
    texture_width: i32,
    frame_width: i32,
    frame_height: i32,
    frame_indices: &[i32],
) -> Vec<Vec2<i32>> {
    let num_columns = texture_width / frame_width;
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
