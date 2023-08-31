use actix::{Actor, StreamHandler};
use actix_web_actors::ws;
use serde::{Deserialize, Serialize};
use serde_json;
use uuid::Uuid;
pub struct Socket {
    pub id: String,
}
impl Actor for Socket {
    type Context = ws::WebsocketContext<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        self.id = Uuid::new_v4().to_string();
        println!("Start");
    }
    // fn
}

#[derive(Serialize, Deserialize)]
struct WsMsg<T> {
    event: String,
    msg: T,
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for Socket {
    fn handle(&mut self, item: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match item {
            Ok(message) => match message {
                ws::Message::Text(text) => {
                    let text_string = text.to_string();
                    let msg_struct = serde_json::from_str::<WsMsg<String>>(&text_string);
                }
                _ => {
                    ctx.text("Unknown format");
                }
            },
            Err(_) => {
                ctx.text("Protocol Error");
            }
        }
    }
}
