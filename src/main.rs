use cc_streaming::{
    cli::ARGS,
    web::{image, stream},
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
    ffmpeg_next::util::log::set_level(ffmpeg_next::log::Level::Warning);

    actix_web::HttpServer::new(|| {
        actix_web::App::new()
            .wrap(actix_web::middleware::Logger::new("[%t] %U %D"))
            .route("/stream", actix_web::web::get().to(stream))
            .route("/image", actix_web::web::get().to(image))
    })
    .bind((std::net::Ipv6Addr::UNSPECIFIED, ARGS.port))
    .unwrap()
    .run()
    .await
    .unwrap();

    Ok(())
}
