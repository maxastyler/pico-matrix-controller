#![feature(adt_const_params)]

use std::{
    marker::PhantomData,
    net::{IpAddr, Ipv6Addr, SocketAddr},
    str::FromStr,
    thread,
};

use axum::{
    body::Body,
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
use tokio::sync::mpsc::Sender;
use tokio::{
    runtime::{Builder, Runtime},
    sync::mpsc::{self, Receiver},
};
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

struct DisplayWindow<State, Message> {
    _state: PhantomData<(State, Message)>,
    pixel_size: u32,
    rows: u32,
    cols: u32,
    pixel_buffer: Vec<[f32; 4]>,
    pixel_offset: f64,
}

impl<State, Message> DisplayWindow<State, Message> {
    pub fn new(pixel_size: u32, rows: u32, cols: u32, pixel_offset: f64) -> Self {
        assert!(pixel_offset <= 1.0);
        Self {
            _state: PhantomData,
            pixel_size,
            rows,
            cols,
            pixel_buffer: vec![[0.0, 0.0, 0.0, 1.0]; (rows * cols) as usize],
            pixel_offset,
        }
    }

    pub fn run(&self, rx: Receiver<Message>) {
        let mut window: PistonWindow = WindowSettings::new(
            "Matrix test server",
            [self.cols * self.pixel_size, self.rows * self.pixel_size],
        )
        .exit_on_esc(true)
        .build()
        .unwrap();

        while let Some(e) = window.next() {
            window.draw_2d(&e, |c, g, _device| {
                clear([1.0; 4], g);
                let offset = self.pixel_size as f64 * self.pixel_offset;
                let square_size = self.pixel_size as f64 * (1.0 - self.pixel_offset * 2.0);
                for ((row, col), colours) in (0..self.rows)
                    .flat_map(|r| {
                        (0..self.cols).map(move |c| {
                            (
                                (r * self.pixel_size) as f64 + offset,
                                (c * self.pixel_size) as f64 + offset,
                            )
                        })
                    })
                    .zip(self.pixel_buffer.iter())
                {
                    rectangle(
                        *colours, // red
                        [col, row, square_size, square_size],
                        c.transform,
                        g,
                    );
                }
            });
        }
    }
}

fn main() {
    let (tx, rx) = mpsc::channel::<()>(10);
    let tokio_rt = spawn_tokio_runtime(tx);

    DisplayWindow::<(), ()>::new(30, 16, 16, 0.3).run(rx);

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
        .route("/api/hello", get(hello))
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
