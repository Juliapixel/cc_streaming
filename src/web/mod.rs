use actix_web::HttpRequest;
use serde::Deserialize;
use ws::StreamWsHandler;

mod decode_actor;
pub mod ws;

#[derive(Debug, Clone, Deserialize)]
pub struct StreamQuery {
    url: url::Url,
    width: u32,
    height: u32,
}

pub async fn stream(
    req: HttpRequest,
    stream: actix_web::web::Payload,
    query: actix_web::web::Query<StreamQuery>,
) -> Result<actix_web::HttpResponse, actix_web::Error> {
    log::debug!("starting stream for {}", &query.url);
    let handler = StreamWsHandler::new(query.url.clone(), query.height);
    let resp = actix_web_actors::ws::start(handler, &req, stream);
    resp
}
