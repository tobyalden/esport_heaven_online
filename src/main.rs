use ggrs::{GGRSError, P2PSession, PlayerType, SessionBuilder, SessionState, UdpNonBlockingSocket};
use instant::{Duration, Instant};
use std::net::SocketAddr;
use structopt::StructOpt;
use tetra::{Context, ContextBuilder, State};
use tetra::graphics::{self, Color, DrawParams, Texture};
use tetra::math::Vec2;
use tetra::time::Timestep;

mod game;
use game::{GGRSConfig, Game};

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
        .with_fps(FPS as usize).unwrap() // (optional) set expected update frequency
        .with_input_delay(1); // (optional) set input delay for the local player

    // add players
    for (i, player_addr) in opt.players.iter().enumerate() {
        // local player
        if player_addr == "localhost" {
            sess_build = sess_build.add_player(PlayerType::Local, i).unwrap();
        } else {
            // remote players
            let remote_addr: SocketAddr = player_addr.parse().unwrap();
            sess_build = sess_build.add_player(PlayerType::Remote(remote_addr), i).unwrap();
        }
    }

    // start the GGRS session
    let socket = UdpNonBlockingSocket::bind_to_port(opt.local_port).unwrap();
    let sess = sess_build.start_p2p_session(socket).unwrap();

    // time variables for tick rate
    let last_update = Instant::now();
    let accumulator = Duration::ZERO;

    ContextBuilder::new("Hello, world!", 320, 180)
        .quit_on_escape(true)
        .vsync(false)
        .timestep(Timestep::Variable)
        .build()?
        .run(|ctx| {
            let mut game = Game::new();
            game.register_local_handles(sess.local_player_handles());

            let resources = Resources::new(ctx);

            Ok(Esport { game, resources, sess, last_update, accumulator })
        })
}

struct Esport {
    game: Game,
    resources: Resources,
    sess: P2PSession<GGRSConfig>,
    last_update: Instant,
    accumulator: Duration,
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
        // if a client is ahead, it will run frames slightly slower to allow catching up
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
            self.accumulator = self.accumulator.saturating_sub(Duration::from_secs_f64(fps_delta));

            // frames are only happening if the self.sessions are synchronized
            if self.sess.current_state() == SessionState::Running {
                // add input for all local players
                for handle in self.sess.local_player_handles() {
                    self.sess.add_local_input(handle, self.game.local_input(ctx, handle)).unwrap();
                }

                match self.sess.advance_frame() {
                    Ok(requests) => self.game.handle_requests(requests),
                    Err(GGRSError::PredictionThreshold) => {
                        println!("Frame {} skipped", self.sess.current_frame())
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
        graphics::clear(ctx, Color::rgb(0.392, 0.584, 0.929));
        self.resources.player_one.draw(
            ctx, DrawParams::new().position(Vec2::new(
                world_to_screen(self.game.state.players[0].x),
                world_to_screen(self.game.state.players[0].y)
            ))
        );
        self.resources.player_two.draw(
            ctx, DrawParams::new().position(Vec2::new(
                world_to_screen(self.game.state.players[1].x),
                world_to_screen(self.game.state.players[1].y)
            ))
        );

        Ok(())
    }
}

fn world_to_screen(coordinate: i32) -> f32 {
    return coordinate as f32 / 1000.0;
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
