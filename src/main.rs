use std::sync::atomic::Ordering;

use cc_streaming::{
    cli::ARGS,
    decoder::{DecodeError, Decoder},
    palette::Palette,
    web::stream,
};
use ffmpeg_next::format::input;
use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, ParallelBridge, ParallelIterator,
};

const DEFAULT_LEVEL: &str = {
    #[cfg(debug_assertions)]
    {
        "DEBUG"
    }
    #[cfg(not(debug_assertions))]
    {
        "INFO"
    }
};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or(DEFAULT_LEVEL));
    ffmpeg_next::init().unwrap();

    actix_web::HttpServer::new(|| {
        actix_web::App::new().route("/stream", actix_web::web::get().to(stream))
    })
    .bind((std::net::Ipv6Addr::UNSPECIFIED, ARGS.port))
    .unwrap()
    .run()
    .await
    .unwrap();

    Ok(())
}
