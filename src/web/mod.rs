use std::{io::Cursor, time::Duration};

use actix_web::HttpRequest;
use either::Either;
use ffmpeg_next::format::input;
use futures::{FutureExt, StreamExt};
use rand::Rng;
use reqwest::header::ACCEPT;
use serde::Deserialize;
use ws::{StreamAudioFrame, StreamVideoFrame};

use crate::{
    decoder::{DecodeError, Decoder},
    dfpwm::DfpwmEncoder,
    dimensions::ResolutionHint,
    ytdl::YtDlpInfo,
};

pub mod ws;

#[derive(Debug, Clone, Deserialize)]
pub struct StreamQuery {
    url: url::Url,
    width: u32,
    height: u32,
}

static REQWEST_CLIENT: std::sync::LazyLock<reqwest::Client> =
    std::sync::LazyLock::new(reqwest::Client::new);

// TODO: make this configurable
static IMAGE_SEMAPHORE: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(32);

pub async fn image(
    query: actix_web::web::Query<StreamQuery>,
) -> Result<actix_web::HttpResponse, actix_web::Error> {
    let _semaphore = IMAGE_SEMAPHORE.acquire().await.expect("this is static cuh");
    let resp = REQWEST_CLIENT
        .get(query.url.clone())
        .header(ACCEPT, "image/png, image/webp, image/gif")
        .send()
        .await
        .map_err(|e| actix_web::error::ErrorNotFound(e.without_url()))?;

    let bytes;

    let format = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|h| image::ImageFormat::from_mime_type(String::from_utf8_lossy(h.as_bytes())))
        .or_else({
            bytes = resp
                .bytes()
                .await
                .map_err(actix_web::error::ErrorBadRequest)?;
            || {
                image::ImageReader::new(Cursor::new(&bytes))
                    .with_guessed_format()
                    .ok()?
                    .format()
            }
        });

    if let Some(format) = format {
        let image =
            tokio::task::spawn_blocking(move || -> Result<StreamVideoFrame, image::ImageError> {
                let image = image::load_from_memory_with_format(&bytes, format)?;

                let res_hint = ResolutionHint::Fit {
                    width: query.width,
                    height: query.height,
                    pixel_aspect: const { 2.0 / 3.0 },
                };
                let (width, height) = res_hint.get_target_res(image.width(), image.height());

                Ok(StreamVideoFrame::from_image_scaled(
                    image.into_rgb8(),
                    width,
                    height,
                ))
            })
            .await
            .unwrap()
            .map_err(actix_web::error::ErrorBadRequest)?;

        Ok(actix_web::HttpResponse::Ok().json(image))
    } else {
        Err(actix_web::error::ErrorBadRequest(
            "failed to determine image format",
        ))
    }
}

// TODO: make this configurable
static STREAM_SEMAPHORE: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(16);

pub async fn stream(
    req: HttpRequest,
    body: actix_web::web::Payload,
    query: actix_web::web::Query<StreamQuery>,
) -> Result<actix_web::HttpResponse, actix_web::Error> {
    let semaphore = STREAM_SEMAPHORE.try_acquire();
    if semaphore.is_err() {
        return Err(actix_web::error::ErrorServiceUnavailable(
            "too many streams open at the same time",
        ));
    }

    log::info!("starting stream for {}", &query.url);
    let (resp, mut session, mut stream) = actix_ws::handle(&req, body)?;

    let (tx, mut rx) = tokio::sync::mpsc::channel(5);

    let ytdl_info = YtDlpInfo::new(&query.url).await?;

    // basically just does all the decoding in regular blocking code and
    // sends it over to the async code via channels (look up to see channel)
    std::thread::spawn({
        let url = ytdl_info
            .best_video_match(query.width, query.height)
            .map(|u| u.parse().unwrap())
            .unwrap();

        move || {
            decode_thread(tx, &url, query.width, query.height);
        }
    });

    // receive frames received from sync code and sends it over to client
    tokio::task::spawn_local({
        let mut session = session.clone();
        async move {
            while let Some(frame) = rx.recv().await {
                match frame {
                    Either::Left(image) => {
                        #[cfg(debug_assertions)]
                        let json = serde_json::to_string_pretty(&image).unwrap();
                        #[cfg(not(debug_assertions))]
                        let json = serde_json::to_string(&image).unwrap();
                        if session.text(json).await.is_err() {
                            break;
                        }
                    }
                    Either::Right(audio) => {
                        #[cfg(debug_assertions)]
                        let json = serde_json::to_string_pretty(&audio).unwrap();
                        #[cfg(not(debug_assertions))]
                        let json = serde_json::to_string(&audio).unwrap();
                        if session.text(json).await.is_err() {
                            break;
                        }
                    }
                }
            }

            log::info!("media over, closing connection");
            let _ = session.close(None).await;
        }
    });

    // handles pinging and ponging
    tokio::task::spawn_local(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut rng = rand::thread_rng();
        let mut last_ping = [0u8; 64];

        loop {
            if futures::select! {
                _ = interval.tick().fuse() => {
                    rng.fill(&mut last_ping);
                    session.ping(&last_ping).await
                },
                msg = stream.next().fuse() => {
                    if let Some(Ok(msg)) = msg {
                        match msg {
                            actix_ws::Message::Text(_) => Ok(()),
                            actix_ws::Message::Binary(_) => Ok(()),
                            actix_ws::Message::Continuation(_) => Ok(()),
                            actix_ws::Message::Ping(ping) => {
                                log::trace!("received ping");
                                session.pong(&ping).await
                            },
                            actix_ws::Message::Pong(_pong) => Ok(()),
                            actix_ws::Message::Close(reason) => {
                                log::info!("session closed: {:?}", reason.map(|r| r.code));

                                break;
                            },
                            actix_ws::Message::Nop => Ok(()),
                        }
                    } else {
                        break
                    }
                }
            }
            .is_err()
            {
                break;
            }
        }

        let _ = session.close(None).await;
    });

    Ok(resp)
}

// boy oh boy that's a nice type
fn decode_thread(
    tx: tokio::sync::mpsc::Sender<Either<StreamVideoFrame, StreamAudioFrame>>,
    url: &url::Url,
    width: u32,
    height: u32,
) {
    // FIXME: allow different urls for audio and video streams and dont require both audio and video
    let ictx = input(url.as_str()).unwrap();
    let vid_stream = ictx
        .streams()
        .best(ffmpeg_next::media::Type::Video)
        .unwrap();
    let aud_stream = ictx
        .streams()
        .best(ffmpeg_next::media::Type::Audio)
        .unwrap();
    let vid_rate: f64 = vid_stream.rate().into();

    log::debug!("video frame rate: {}", vid_rate);

    let decoder = Decoder::new_video_only(
        vid_stream,
        ResolutionHint::fit(width, height, const { 2.0 / 3.0 }),
    )
    .unwrap();

    let mut decode_iter = decoder.into_frame_iter(ictx);

    let mut dfpwm_encoder = DfpwmEncoder::new();
    loop {
        match decode_iter.next() {
            Some(Ok(Either::Left(video_frame))) => {
                if tx
                    .blocking_send(Either::Left(StreamVideoFrame::from_image(
                        video_frame.image(),
                    )))
                    .is_err()
                {
                    break;
                }
            }
            Some(Ok(Either::Right(audio_frame))) => {
                if tx
                    .blocking_send(Either::Right(StreamAudioFrame {
                        samples: dfpwm_encoder.encode(audio_frame.samples().iter().copied()),
                    }))
                    .is_err()
                {
                    break;
                }
            }
            Some(Err(DecodeError::NoFramesYet)) => (),
            Some(Err(e)) => {
                log::error!("{e}");
                break;
            }
            None => break,
        }
    }
}
