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
use piston_window::*;
use std::{
    net::{IpAddr, Ipv6Addr, SocketAddr},
    str::FromStr,
    thread,
};
use tokio::runtime::{Builder, Runtime};
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

fn main() {
    let tokio_rt = spawn_tokio_runtime();

    let mut window: PistonWindow = WindowSettings::new("Hello Piston!", [640, 480])
        .exit_on_esc(true)
        .build()
        .unwrap();
    while let Some(e) = window.next() {
        window.draw_2d(&e, |c, g, _device| {
            clear([1.0; 4], g);
            rectangle(
                [1.0, 0.0, 0.0, 1.0], // red
                [0.0, 0.0, 100.0, 100.0],
                c.transform,
                g,
            );
        });
    }
    tokio_rt.shutdown_background();
}

fn spawn_tokio_runtime() -> Runtime {
    let runtime = Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap();

    runtime.spawn(start_app());
    runtime
}

async fn start_app() {
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
