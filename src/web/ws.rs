use actix::{Actor, AsyncContext, StreamHandler};
use actix_web_actors::ws;
use ffmpeg_next::format::input;
use serde::Serialize;
use url::Url;

use crate::decoder::Decoder;

use super::decode_actor::DecodeActor;

pub struct StreamWsHandler {
    url: Url,
    height: u32,
}

impl StreamWsHandler {
    pub fn new(url: impl Into<Url>, height: u32) -> Self {
        Self {
            url: url.into(),
            height,
        }
    }
}

impl Actor for StreamWsHandler {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let ictx = input(self.url.as_str()).unwrap();
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

        let decoder = Decoder::new(vid_stream, aud_stream, self.height).unwrap();

        let mut frame_iter = decoder.into_frame_iter(ictx);

        ctx.spawn(DecodeActor::new(frame_iter));
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct StreamVideoFrame {
    pub palette: Vec<[u8; 3]>,
    pub rows: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StreamAudioFrame {
    pub samples: Vec<u8>,
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for StreamWsHandler {
    fn handle(&mut self, item: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match item {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(msg)) => (),
            Ok(_) => ctx.close(None),
            Err(_) => ctx.close(Some(ws::CloseReason {
                code: ws::CloseCode::Protocol,
                description: None,
            })),
        }
    }
}
