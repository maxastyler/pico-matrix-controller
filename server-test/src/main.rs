use std::{
    net::{IpAddr, Ipv6Addr, SocketAddr},
    str::FromStr,
};

use axum::{
    body::Body,
    http::{Response, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use clap::Parser;
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

#[tokio::main]
async fn main() {
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
