use actix::{
    dev::MessageResponse, Actor, Addr, AsyncContext, Handler, MailboxError, Message, StreamHandler,
};
use actix_web_actors::ws;
use once_cell::sync::Lazy;
use rand::{self, Rng};
use serde::{Deserialize, Serialize};
use serde_json;
use std::{collections::HashMap, sync::Mutex};
// use std::vec;
use futures::executor::block_on;
use uuid::Uuid;

// let mut rooms = vec
static SERVER: Server = Server {
    rooms: Vec::new(),
    addr: None,
};

static ROOMS: Lazy<Mutex<Vec<Room>>> = Lazy::new(|| {
    let map: Vec<Room> = Vec::new();
    return Mutex::new(map);
});

#[derive(Message)]
#[rtype(result = "ServerRes")]

enum ServerCommands {
    AddRoom(Socket),
    AddPlayerToRoom(Socket, u16),
    SendMsgToSocket(Socket, MSG),
}
#[derive(Message)]
#[rtype(result = "()")]
#[derive(MessageResponse)]
enum ServerRes {
    AddRoom(u16),
    AddPlayerToRoom(Option<String>),
    SendMsgToSocket(bool),
}

// impl MessageResponse<Server, ServerRes> for ServerRes {
//     fn handle(
//         self,
//         ctx: &mut <Server as Actor>::Context,
//         tx: Option<actix::dev::OneshotSender<<ServerRes as Message>::Result>>,
//     ) {
//     }
// }

pub struct Server {
    pub rooms: Vec<Room>,
    pub addr: Option<Addr<Server>>,
}

impl Server {
    fn find_room(&mut self, key: u16) -> Option<&mut Room> {
        for room in self.rooms.iter_mut() {
            if room.id == key {
                return Some(room);
            }
        }
        return None;
    }
}

impl Handler<ServerCommands> for Server {
    type Result = ServerRes;
    fn handle(&mut self, msg: ServerCommands, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            ServerCommands::AddRoom(p1_socket) => {
                let mut rng = rand::thread_rng();
                let mut room_code = rng.gen::<u16>();
                println!("Here1");

                while let Some(_room) = self.find_room(room_code) {
                    println!("Here2");
                    room_code = rng.gen::<u16>();
                }
                let room = Room::init(room_code, p1_socket, None);
                self.rooms.push(room);
                println!("roomcode = {}", room_code);
                return ServerRes::AddRoom(room_code);
            }
            ServerCommands::AddPlayerToRoom(p2_socket, room_id) => {
                let room = &mut self.find_room(room_id);
                if let Some(room) = room {
                    if let Some(_) = room.sockets.1 {
                        return ServerRes::AddPlayerToRoom(None);
                    } else {
                        room.add_player(p2_socket.clone());
                        let msg = MSG::init(Event::ConnectWith, &p2_socket.id);
                        room.sockets.clone().0.addr.unwrap().do_send(msg);
                        return ServerRes::AddPlayerToRoom(Some(String::from(&room.sockets.0.id)));
                    }
                } else {
                    return ServerRes::AddPlayerToRoom(None);
                }
            }
            ServerCommands::SendMsgToSocket(sckt, msg) => {
                let socket = sckt.addr.unwrap();
                socket.do_send(msg);
                return ServerRes::SendMsgToSocket(true);
            }
        }
    }
}

impl Actor for Server {
    type Context = actix::Context<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        println!("Server Started");
        self.addr = Some(ctx.address());
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct MSG {
    event: Event,
    message: String,
}

impl MSG {
    fn init(event: Event, message: &String) -> Self {
        return MSG {
            event,
            message: String::from(message),
        };
    }
}

#[derive(Message)]
#[rtype(result = "Socket")]
struct GetSocket {}

impl Clone for MSG {
    fn clone(&self) -> Self {
        return MSG {
            event: self.event.clone(),
            message: self.message.clone(),
        };
    }
}

fn find_room_from_key<'a>(key: u16, rooms: &'a mut Vec<Room>) -> Option<&'a mut Room> {
    // let rooms = ROOMS.lock().unwrap().as_slice();
    for room in rooms.iter_mut() {
        if room.id == key {
            println!("{:p}", room);
            return Some(room);
        }
    }
    return None;
}

pub struct Room {
    pub id: u16,
    pub sockets: (Socket, Option<Socket>),
}

impl Clone for Room {
    fn clone(&self) -> Self {
        return Room {
            id: self.id.clone(),
            sockets: self.sockets.clone(),
        };
    }
}

impl Room {
    fn init(id: u16, p1_socket: Socket, p2_socket: Option<Socket>) -> Room {
        return Room {
            id,
            sockets: (p1_socket.clone(), p2_socket.clone()),
        };
    }
    fn add_player(&mut self, pl_socket: Socket) {
        self.sockets.1 = Some(pl_socket.clone());
    }
    fn get_pl1_addr(&mut self) -> Addr<Socket> {
        return self.sockets.0.clone().addr.unwrap();
    }
    fn get_pl2_addr(&mut self) -> Addr<Socket> {
        return self.sockets.1.clone().unwrap().addr.unwrap();
    }
    fn display(&self) {
        let p1id = self.sockets.0.clone().id;
        let pl2id = if let Some(ref s) = self.sockets.1 {
            s.id.clone()
        } else {
            String::from("None")
        };
        println!("{} {}", p1id, pl2id);
    }
}

#[derive(MessageResponse)]
pub struct Socket {
    pub id: String,
    pub addr: Option<Addr<Socket>>, // pub server: Addr<Server>,
    pub server: Addr<Server>,
}
impl Handler<MSG> for Socket {
    type Result = ();
    fn handle(&mut self, msg: MSG, ctx: &mut Self::Context) -> Self::Result {
        let event = msg.event;
        let msg = msg.message;
        let res = create_ws_msg(EventOrError::Event(event), &msg).unwrap();
        println!("{}", res);
        ctx.text(res);
    }
}

impl Handler<GetSocket> for Socket {
    type Result = Socket;
    fn handle(&mut self, _msg: GetSocket, _ctx: &mut Self::Context) -> Self::Result {
        return self.clone();
    }
}

impl Clone for Socket {
    fn clone(&self) -> Self {
        Socket {
            id: self.id.clone(),
            addr: self.addr.clone(),
            server: self.server.clone(),
        }
    }
}

impl Actor for Socket {
    type Context = ws::WebsocketContext<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        self.id = Uuid::new_v4().to_string();
        self.addr = Some(ctx.address());
        println!("Start");
    }

    // fn
}
#[derive(Serialize, Deserialize)]
enum Event {
    Move,
    GameOver,
    Start,
    GetCode,
    ConnectWith,
    OppReady,
}
enum EventError {
    ParseError,
    InvalidCode,
    RoomFull,
}

impl Clone for Event {
    fn clone(&self) -> Self {
        match self {
            Event::Start => Event::Start,
            Event::GameOver => Event::GameOver,
            Event::GetCode => Event::GetCode,
            Event::ConnectWith => Event::ConnectWith,
            Event::Move => Event::Move,
            Event::OppReady => Event::OppReady,
        }
    }
}

impl EventError {
    fn to_string(&self) -> String {
        match self {
            EventError::ParseError => return String::from("Parsing Error"),
            EventError::InvalidCode => return String::from("Invalid Code"),
            EventError::RoomFull => return String::from("Room Full"),
        }
    }
}

impl Event {
    fn to_string(&self) -> String {
        match self {
            Event::Move => return String::from("Move"),
            Event::GameOver => return String::from("GameOver"),
            Event::Start => return String::from("Start"),
            Event::ConnectWith => return String::from("ConnectWith"),
            Event::GetCode => return String::from("GetCode"),
            Event::OppReady => return String::from("OppReady"),
        }
    }
    fn from_string(string: &str) -> Result<Event, EventError> {
        match string {
            "Move" => return Ok(Event::Move),
            "GameOver" => return Ok(Event::GameOver),
            "Start" => return Ok(Event::Start),
            "ConnectWith" => return Ok(Event::ConnectWith),
            "GetCode" => return Ok(Event::GetCode),
            "OppReady" => return Ok(Event::OppReady),
            _ => return Err(EventError::ParseError),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct WsMsg<E, M> {
    event: E,
    msg: M,
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for Socket {
    fn handle(&mut self, item: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match item {
            Ok(message) => match message {
                ws::Message::Text(text) => {
                    let text_string = text.to_string();
                    let msg_struct = serde_json::from_str::<WsMsg<String, String>>(&text_string);
                    match msg_struct {
                        Ok(socket_msg) => {
                            let event = socket_msg.event;
                            let msg = socket_msg.msg;
                            match Event::from_string(&event) {
                                Ok(event) => match event {
                                    Event::Move => ctx.text("moving"),
                                    Event::GameOver => ctx.text("gameover"),
                                    Event::Start => ctx.text("starting"),
                                    Event::ConnectWith => {
                                        let room_code = msg.parse::<u16>();
                                        match room_code {
                                            Ok(code) => {
                                                async fn temp(
                                                    sckt: Socket,
                                                    code: u16,
                                                ) -> Result<ServerRes, MailboxError>
                                                {
                                                    return sckt
                                                        .server
                                                        .send(ServerCommands::AddPlayerToRoom(
                                                            sckt.clone(),
                                                            code,
                                                        ))
                                                        .await;
                                                }
                                                let res =
                                                    block_on(temp(self.clone(), code)).unwrap();
                                                if let ServerRes::AddPlayerToRoom(pl1_id) = res {
                                                    if let Some(pl1_id) = pl1_id {
                                                        let msg = create_ws_msg(
                                                            EventOrError::Event(Event::ConnectWith),
                                                            &pl1_id,
                                                        )
                                                        .unwrap();
                                                        ctx.text(msg);
                                                    } else {
                                                        let msg = create_ws_msg(
                                                            EventOrError::EventError(
                                                                EventError::RoomFull,
                                                            ),
                                                            &"Couldn't add player",
                                                        )
                                                        .unwrap();
                                                        ctx.text(msg);
                                                    }
                                                } else {
                                                    let msg = create_ws_msg(
                                                        EventOrError::EventError(
                                                            EventError::RoomFull,
                                                        ),
                                                        &"Couldn't add player",
                                                    )
                                                    .unwrap();
                                                    ctx.text(msg);
                                                }
                                            }
                                            Err(_) => {
                                                let msg = create_ws_msg(
                                                    EventOrError::EventError(
                                                        EventError::InvalidCode,
                                                    ),
                                                    &"Invalid room code",
                                                )
                                                .unwrap();
                                                ctx.text(msg);
                                            }
                                        }
                                    }
                                    Event::GetCode => {
                                        async fn abcd(
                                            sckt: Socket,
                                        ) -> Result<ServerRes, MailboxError>
                                        {
                                            println!("Blocking");
                                            return sckt
                                                .server
                                                .send(ServerCommands::AddRoom(sckt.clone()))
                                                .await;
                                        }
                                        let x = block_on(abcd(self.clone())).unwrap();
                                        if let ServerRes::AddRoom(room_code) = x {
                                            println!("Here");
                                            let res = create_ws_msg(
                                                EventOrError::Event(Event::GetCode),
                                                &HashMap::from([
                                                    ("id", &self.id),
                                                    ("code", &room_code.to_string()),
                                                ]),
                                            )
                                            .unwrap();
                                            ctx.text(res);
                                        } else {
                                            let res = create_ws_msg(
                                                EventOrError::EventError(EventError::ParseError),
                                                &"Internal Error",
                                            )
                                            .unwrap();
                                            ctx.text(res);
                                        }
                                    }
                                    Event::OppReady => {
                                        println!("{}", msg);
                                        let code = msg.trim().parse::<u16>();
                                        println!("Opp Ready");
                                        if let Ok(code) = code {
                                            let rooms = &mut ROOMS.lock().unwrap().to_vec();
                                            let room = find_room_from_key(code, rooms);
                                            if let Some(room) = room {
                                                println!("{:p}", room);
                                                room.display();
                                                if let Some(pl2_socket) = room.sockets.1.clone() {
                                                    if self.id == pl2_socket.id {
                                                        room.clone().get_pl1_addr().clone().do_send(
                                                            MSG::init(
                                                                Event::OppReady,
                                                                &pl2_socket.id,
                                                            ),
                                                        )
                                                    } else {
                                                        room.clone().get_pl2_addr().clone().do_send(
                                                            MSG::init(
                                                                Event::OppReady,
                                                                &room.sockets.0.id,
                                                            ),
                                                        )
                                                    }
                                                } else {
                                                    let text = create_ws_msg(
                                                        EventOrError::EventError(
                                                            EventError::RoomFull,
                                                        ),
                                                        &"Player 2 not joined",
                                                    )
                                                    .unwrap();
                                                    ctx.text(text)
                                                }
                                                // room.sockets.0.id;
                                            } else {
                                                let msg = create_ws_msg(
                                                    EventOrError::EventError(
                                                        EventError::ParseError,
                                                    ),
                                                    &"Invalid Room Code",
                                                )
                                                .unwrap();
                                                ctx.text(msg);
                                            }
                                        } else {
                                            let msg = create_ws_msg(
                                                EventOrError::EventError(EventError::ParseError),
                                                &"Invalid Room Code",
                                            )
                                            .unwrap();
                                            ctx.text(msg);
                                        }
                                    }
                                },
                                Err(event_error) => match event_error {
                                    EventError::ParseError => {
                                        let res = create_ws_msg(
                                            EventOrError::EventError(event_error),
                                            &"Error While Parsing Event",
                                        )
                                        .unwrap();
                                        ctx.text(res);
                                    }
                                    _ => {}
                                },
                            }
                        }
                        Err(_err) => {
                            let res = create_ws_msg(
                                EventOrError::EventError(EventError::ParseError),
                                &"Error parsing json",
                            )
                            .unwrap();
                            ctx.text(res);
                        }
                    }
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

enum EventOrError {
    Event(Event),
    EventError(EventError),
}

fn create_ws_msg<T>(event: EventOrError, msg: &T) -> Result<String, serde_json::Error>
where
    T: Serialize,
{
    match event {
        EventOrError::Event(event) => {
            return serde_json::to_string(&WsMsg {
                event: event.to_string(),
                msg,
            })
        }
        EventOrError::EventError(err) => {
            return serde_json::to_string(&WsMsg {
                event: err.to_string(),
                msg,
            })
        }
    }
}
