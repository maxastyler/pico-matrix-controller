#![feature(adt_const_params)]

use matrix_state::{MatrixDisplay, RGB8};
use std::{
    marker::PhantomData,
    net::{IpAddr, Ipv6Addr, SocketAddr},
    str::FromStr,
    thread,
    time::Duration,
};

use axum::{
    body::Body,
    extract::{ws::WebSocket, ConnectInfo, WebSocketUpgrade},
    http::{Response, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use clap::Parser;
use macroquad::prelude::*;
use macroquad::ui::widgets::Window;
use piston_window::ellipse::circle;
use piston_window::*;
use tokio::{
    runtime::{Builder, Runtime},
    sync::mpsc::{self, Receiver},
};
use tokio::{sync::mpsc::Sender, time::sleep};
use tower::{ServiceBuilder, ServiceExt};
use tower_http::{services::ServeDir, trace::TraceLayer};

#[derive(Parser, Debug)]
#[clap(
    name = "server",
    about = "A server for the pico matrix controller project"
)]
struct Opt {
    #[clap(short = 'a', long = "addr", default_value = "::1")]
    addr: String,
    #[clap(short = 'p', long = "port", default_value = "8080")]
    port: u16,
    #[clap(short = 'l', long = "log", default_value = "debug")]
    log_level: String,
    #[clap(long = "static-dir", default_value = "./dist")]
    static_dir: String,
}

struct DisplayWindow<State, Message, const ROWS: usize, const COLS: usize> {
    _state: PhantomData<(State, Message)>,
    pixel_size: u32,
    pixel_buffer: Vec<RGB8>,
    pixel_offset: f64,
}

impl<State, Message, const ROWS: usize, const COLS: usize> MatrixDisplay
    for DisplayWindow<State, Message, ROWS, COLS>
{
    fn get_mut(&mut self, row: usize, col: usize) -> Option<&mut RGB8> {
        self.pixel_buffer.get_mut(row * COLS + col)
    }

    fn get(&self, row: usize, col: usize) -> Option<&RGB8> {
        self.pixel_buffer.get(row * COLS + col)
    }

    fn size(&self) -> (usize, usize) {
        (ROWS, COLS)
    }
}

impl<State, Message, const ROWS: usize, const COLS: usize>
    DisplayWindow<State, Message, ROWS, COLS>
{
    pub fn new(pixel_size: u32, pixel_offset: f64) -> Self {
        assert!(pixel_offset <= 1.0);
        Self {
            _state: PhantomData,
            pixel_size,
            pixel_buffer: vec![
                RGB8 {
                    padding: 0,
                    r: 0,
                    g: 0,
                    b: 0
                };
                (ROWS * COLS) as usize
            ],
            pixel_offset,
        }
    }

    pub fn run(&mut self, rx: Receiver<Message>) {
        let mut window: PistonWindow = WindowSettings::new(
            "Matrix test server",
            [COLS as u32 * self.pixel_size, ROWS as u32 * self.pixel_size],
        )
        .exit_on_esc(true)
        .build()
        .unwrap();

        let pixel_size = self.pixel_size;

        while let Some(e) = window.next() {
            window.draw_2d(&e, |c, g, _device| {
                clear([1.0; 4], g);
                let offset = self.pixel_size as f64 * self.pixel_offset;
                let square_size = self.pixel_size as f64 * (1.0 - self.pixel_offset * 2.0);
                for ((row, col), colour) in (0..ROWS)
                    .flat_map(|r| {
                        (0..COLS).map(move |c| {
                            (
                                (r as u32 * pixel_size) as f64 + offset,
                                (c as u32 * pixel_size) as f64 + offset,
                            )
                        })
                    })
                    .zip(self.pixel_buffer.iter())
                {
                    rectangle(
                        transform_colour(*colour),
                        [col, row, square_size, square_size],
                        c.transform,
                        g,
                    );
                }
            });
        }
    }
}

fn transform_colour(RGB8 { r, b, g, padding }: RGB8) -> [f32; 4] {
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0]
}

fn main() {
    let (tx, rx) = mpsc::channel::<()>(10);
    let tokio_rt = spawn_tokio_runtime(tx);

    DisplayWindow::<(), (), 16, 16>::new(30, 0.3).run(rx);

    tokio_rt.shutdown_background();
}

fn spawn_tokio_runtime<Message: Send + 'static>(tx: Sender<Message>) -> Runtime {
    let runtime = Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap();

    runtime.spawn(start_app(tx));
    runtime
}

async fn start_app<Message>(tx: Sender<Message>) {
    let opt = Opt::parse();

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", format!("{},hyper=info,mio=info", opt.log_level))
    }

    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/ws/ws", get(ws_handler))
        .fallback_service(get(|req| async move {
            ServeDir::new(opt.static_dir).oneshot(req).await
        }))
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()));

    let sock_addr = SocketAddr::from((
        IpAddr::from_str(opt.addr.as_str()).unwrap_or(IpAddr::V6(Ipv6Addr::LOCALHOST)),
        opt.port,
    ));

    log::info!("listening on http://{}", sock_addr);

    axum_server::bind(sock_addr)
        .serve(app.into_make_service())
        .await
        .expect("Unable to start server");
}

async fn hello() -> impl IntoResponse {
    "hello from server!"
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, addr))
}

async fn handle_socket(mut socket: WebSocket, who: SocketAddr) {
    loop {
        sleep(Duration::from_secs(1)).await;
        socket
            .send(axum::extract::ws::Message::Text("Hiiii".into()))
            .await
            .unwrap();
    }
}
