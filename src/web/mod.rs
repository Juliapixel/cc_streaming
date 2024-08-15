use std::time::Duration;

use actix_web::HttpRequest;
use either::Either;
use ffmpeg_next::format::input;
use futures::{FutureExt, StreamExt};
use rand::Rng;
use serde::Deserialize;
use ws::{StreamAudioFrame, StreamVideoFrame};

use crate::{
    decoder::{DecodeError, Decoder},
    dfpwm::DfpwmEncoder,
    dimensions::ResolutionHint,
    palette::Palette,
    ytdl::get_stream_url,
};

pub mod ws;

#[derive(Debug, Clone, Deserialize)]
pub struct StreamQuery {
    url: url::Url,
    width: u32,
    height: u32,
}

pub async fn stream(
    req: HttpRequest,
    body: actix_web::web::Payload,
    query: actix_web::web::Query<StreamQuery>,
) -> Result<actix_web::HttpResponse, actix_web::Error> {
    log::debug!("starting stream for {}", &query.url);
    let (resp, mut session, mut stream) = actix_ws::handle(&req, body)?;

    let (tx, mut rx) = tokio::sync::mpsc::channel(5);

    // basically just does all the decoding in regular blocking code and
    // sends it over to the async code via channels (look up to see channel)
    tokio::spawn(async move {
        let url = get_stream_url(&query.url).await;
        std::thread::spawn(move || {
            decode_thread(tx, url.first().unwrap(), query.width, query.height)
        })
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
                            actix_ws::Message::Pong(pong) => todo!("ponging"),
                            actix_ws::Message::Close(_) => break,
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
                let palette = Palette::new(16, &video_frame);

                let mut lines: Vec<String> = (0..video_frame.height())
                    .map(|_| String::with_capacity(video_frame.width() as usize))
                    .collect();

                for (i, pal_idx) in palette.index_iter(&video_frame).enumerate() {
                    let line = i / video_frame.width() as usize;
                    lines[line].push(char::from_digit(pal_idx as u32, 16).unwrap());
                }

                if tx
                    .blocking_send(Either::Left(StreamVideoFrame {
                        palette: palette
                            .into_iter()
                            .map(|pix| [pix.0[0], pix.0[1], pix.0[2]])
                            .collect(),
                        rows: lines,
                    }))
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
