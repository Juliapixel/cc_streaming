use cc_streaming::{
    cli::ARGS,
    web::{image, stream},
};
use log::Level;
use scuffle_ffmpeg::log::LogLevel;

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
    scuffle_ffmpeg::log::log_callback_set(
        |mut level: LogLevel, class: Option<String>, msg: String| {
            let class = class.as_deref().unwrap_or("ffmpeg");

            // We purposely ignore this message because it's a false positive
            if msg == "deprecated pixel format used, make sure you did set range correctly" {
                level = LogLevel::Debug;
            }

            let mut level = match level {
                LogLevel::Trace => log::Level::Trace,
                LogLevel::Verbose => log::Level::Trace,
                LogLevel::Debug => log::Level::Debug,
                LogLevel::Info => log::Level::Info,
                LogLevel::Warning => log::Level::Warn,
                LogLevel::Quiet => log::Level::Trace,
                LogLevel::Error => log::Level::Error,
                LogLevel::Panic => log::Level::Error,
                LogLevel::Fatal => log::Level::Error,
                LogLevel(_) => log::Level::Debug,
            };

            if cfg!(not(debug_assertions)) {
                level = match level {
                    Level::Error => Level::Warn,
                    Level::Warn => Level::Info,
                    Level::Info => Level::Debug,
                    Level::Debug => Level::Trace,
                    Level::Trace => Level::Trace,
                }
            }

            log::log!(level, "{class} @ {msg}");
        },
    );

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
