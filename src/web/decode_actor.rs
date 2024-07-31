use std::{future::Future, task::Poll};

use actix::ActorFuture;
use either::Either;
use image::RgbImage;
use tokio::sync::mpsc::Receiver;

use crate::{
    decoder::{iter::DecodeIter, DecodeError},
    dfpwm::DfwpmEncoder,
    palette::Palette,
};

use super::ws::{StreamAudioFrame, StreamVideoFrame, StreamWsHandler};

pub struct DecodeActor {
    frame_receiver: Receiver<Either<StreamVideoFrame, StreamAudioFrame>>,
}

impl DecodeActor {
    pub fn new(mut decode_iter: DecodeIter) -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel(5);
        std::thread::spawn(move || loop {
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
                                .iter()
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
                            samples: DfwpmEncoder::encode(&audio_frame),
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
        });
        Self { frame_receiver: rx }
    }
}

impl ActorFuture<StreamWsHandler> for DecodeActor {
    type Output = ();

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        srv: &mut StreamWsHandler,
        ctx: &mut <StreamWsHandler as actix::Actor>::Context,
        task: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match std::pin::pin!(self.frame_receiver.recv()).poll(task) {
            Poll::Ready(Some(Either::Left(video_frame))) => {
                ctx.text(serde_json::to_string_pretty(&video_frame).unwrap());
                task.waker().wake_by_ref();
                Poll::Pending
            }
            Poll::Ready(Some(Either::Right(audio_frame))) => {
                ctx.text(serde_json::to_string_pretty(&audio_frame).unwrap());
                task.waker().wake_by_ref();
                Poll::Pending
            }
            Poll::Ready(None) => {
                ctx.close(Some(actix_web_actors::ws::CloseReason {
                    code: actix_web_actors::ws::CloseCode::Normal,
                    description: Some("media over".into()),
                }));
                Poll::Ready(())
            }
            Poll::Pending => Poll::Pending,
        }
    }
}
