use actix::{dev::MessageResponse, Actor, Addr, AsyncContext, Handler, Message, StreamHandler};
use actix_web_actors::ws;
use rand::{self, Rng};
use serde::{Deserialize, Serialize};
use serde_json::{self, Value};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Message)]
#[rtype(result = "()")]

enum ServerCommands {
    AddRoom(Socket),
    AddPlayerToRoom(Socket, u16),
    OppReady(Socket, u16),
    Move(Addr<Socket>, String, u16, (u8, u8), (u8, u8)),
    Promote(u16, String, (u8, u8), String),
}

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
    type Result = ();
    fn handle(&mut self, msg: ServerCommands, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            ServerCommands::AddRoom(p1_socket) => {
                let mut rng = rand::thread_rng();
                let mut room_code = rng.gen::<u16>();
                while let Some(_room) = self.find_room(room_code) {
                    room_code = rng.gen::<u16>();
                }
                let room = Room::init(room_code, p1_socket.clone(), None, p1_socket.clone().id);
                self.rooms.push(room);
                #[derive(Serialize)]
                struct IdAndCode {
                    id: String,
                    code: String,
                }
                p1_socket.clone().addr.unwrap().do_send(MSG::init(
                    EventOrError::Event(Event::GetCode),
                    &serde_json::to_string(&IdAndCode {
                        id: p1_socket.id,
                        code: room_code.to_string(),
                    })
                    .unwrap(),
                ));
                println!("roomcode = {}", room_code);
            }
            ServerCommands::AddPlayerToRoom(p2_socket, room_id) => {
                let room = &mut self.find_room(room_id);
                if let Some(room) = room {
                    println!("Here room");
                    room.display();
                    if let Some(_) = room.sockets.1 {
                        p2_socket.clone().addr.unwrap().do_send(MSG::init(
                            EventOrError::EventError(EventError::RoomFull),
                            &String::from("Room Full"),
                        ));
                    } else {
                        room.add_player(p2_socket.clone());
                        let msg =
                            MSG::init(EventOrError::Event(Event::ConnectWith), &p2_socket.name);
                        room.sockets.clone().0.addr.unwrap().do_send(msg);
                        p2_socket.addr.unwrap().do_send(MSG::init(
                            EventOrError::Event(Event::ConnectWith),
                            &room.sockets.clone().0.name,
                        ));
                    }
                } else {
                    let msg = MSG::init(
                        EventOrError::EventError(EventError::RoomFull),
                        &String::from("No room found"),
                    );
                    p2_socket.addr.unwrap().do_send(msg);
                }
            }
            ServerCommands::OppReady(sckt, code) => {
                let room = &mut self.find_room(code);
                if let Some(room) = room {
                    if let Some(pl2_socket) = room.sockets.1.clone() {
                        if sckt.id == pl2_socket.id {
                            room.get_pl1_addr()
                                .do_send(MSG::init(EventOrError::Event(Event::OppReady), &sckt.id));
                        } else {
                            room.get_pl2_addr().do_send(MSG::init(
                                EventOrError::Event(Event::OppReady),
                                &room.sockets.0.id,
                            ));
                        }
                    } else {
                        let msg = MSG::init(
                            EventOrError::EventError(EventError::RoomFull),
                            &String::from("Room is full"),
                        );
                        sckt.addr.unwrap().do_send(msg);
                    }
                } else {
                    let msg = MSG::init(
                        EventOrError::EventError(EventError::RoomFull),
                        &String::from("No room found"),
                    );
                    // let e = MSG::init(
                    sckt.addr.unwrap().do_send(msg);
                }
            }
            ServerCommands::Move(addr, socket_id, code, (i, j), (k, l)) => {
                if i < 8 && j < 8 && k < 8 && l < 8 && (i != k || j != l) {
                    let room = &mut self.find_room(code);
                    if let Some(room) = room {
                        if let Some(_) = room.get_addr_from_id(socket_id.clone()) {
                            println!("{} {}", socket_id, room.turn);
                            if socket_id == room.turn {
                                if let Some(sib_sckt) = room.get_sibling_sckt(socket_id) {
                                    room.turn = String::from(&sib_sckt.id);
                                    let msg = MSG::init(
                                        EventOrError::Event(Event::Move),
                                        &serde_json::to_string(&HashMap::from([
                                            ("i", i),
                                            ("j", j),
                                            ("k", k),
                                            ("l", l),
                                        ]))
                                        .unwrap(),
                                    );
                                    sib_sckt.addr.unwrap().do_send(msg);
                                } else {
                                }
                            } else {
                                let msg = MSG::init(
                                    EventOrError::EventError(EventError::RoomFull),
                                    &String::from("Not your turn"),
                                );
                                addr.do_send(msg);
                            }
                        } else {
                            let msg = MSG::init(
                                EventOrError::EventError(EventError::RoomFull),
                                &String::from("You Are not in room"),
                            );
                            addr.do_send(msg);
                        }
                    } else {
                        let msg = MSG::init(
                            EventOrError::EventError(EventError::RoomFull),
                            &String::from("No room found"),
                        );
                        addr.do_send(msg)
                    }
                } else {
                    let msg = MSG::init(
                        EventOrError::EventError(EventError::RoomFull),
                        &String::from("Invalid Move"),
                    );
                    addr.do_send(msg)
                }
            }

            ServerCommands::Promote(room_code, sckt_id, (i, j), value) => {
                let room = &mut self.find_room(room_code);
                if let Some(room) = room {
                    if room.turn == sckt_id {
                        let sib_sckt = room.get_sibling_sckt(sckt_id).unwrap();
                        room.turn = sib_sckt.id;
                        sib_sckt.addr.unwrap().do_send(MSG::init(
                            EventOrError::Event(Event::Promote),
                            &serde_json::to_string(&HashMap::from([
                                ("i", i.to_string()),
                                ("j", j.to_string()),
                                ("value", value),
                            ]))
                            .unwrap(),
                        ));
                    } else {
                        room.get_addr_from_id(sckt_id).unwrap().do_send(MSG {
                            event: EventOrError::EventError(EventError::RoomFull),
                            message: String::from("Not your turn"),
                        })
                    }
                }
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
    event: EventOrError,
    message: String,
}

impl MSG {
    fn init(event: EventOrError, message: &String) -> Self {
        return MSG {
            event,
            message: String::from(message),
        };
    }
}

#[derive(Message, Clone)]
#[rtype(result = "Socket")]
struct GetSocket {}

#[derive(Clone)]
pub struct Room {
    pub id: u16,
    pub turn: String,
    pub sockets: (Socket, Option<Socket>),
}

impl Room {
    fn init(id: u16, p1_socket: Socket, p2_socket: Option<Socket>, turn: String) -> Room {
        return Room {
            id,
            sockets: (p1_socket.clone(), p2_socket.clone()),
            turn,
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
    fn get_addr_from_id(&mut self, sckt_id: String) -> Option<Addr<Socket>> {
        if sckt_id == self.sockets.0.id {
            return Some(self.sockets.clone().0.addr.unwrap());
        } else {
            if let Some(pl2) = self.sockets.1.clone() {
                if sckt_id == pl2.id {
                    return Some(pl2.addr.unwrap());
                }
            }
        }
        return None;
    }
    fn get_sibling_sckt(&mut self, sckt_id: String) -> Option<Socket> {
        if sckt_id == self.sockets.0.id {
            return self.sockets.1.clone();
        } else {
            if let Some(pl2) = self.sockets.1.clone() {
                if sckt_id == pl2.id {
                    return Some(self.sockets.0.clone());
                }
            }
        }
        return None;
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

#[derive(MessageResponse, Clone)]
pub struct Socket {
    pub id: String,
    pub name: String,
    pub addr: Option<Addr<Socket>>, // pub server: Addr<Server>,
    pub server: Addr<Server>,
}

impl Socket {
    fn set_name(&mut self, name: String) {
        self.name = String::from(&name);
    }
}

impl Handler<MSG> for Socket {
    type Result = ();
    fn handle(&mut self, msg: MSG, ctx: &mut Self::Context) -> Self::Result {
        let event: String = match msg.event {
            EventOrError::Event(e) => e.to_string(),
            EventOrError::EventError(e) => e.to_string(),
        };
        let msg = msg.message;
        let x: Result<Value, serde_json::Error> = serde_json::from_str(&msg.clone());
        if let Ok(y) = x {
            let res = serde_json::to_string(&WsMsg { event, msg: y }).unwrap();
            println!("{}", res);
            ctx.text(res);
        } else {
            let res = serde_json::to_string(&WsMsg { event, msg }).unwrap();
            println!("{}", res);
            ctx.text(res);
        }
    }
}

impl Handler<GetSocket> for Socket {
    type Result = Socket;
    fn handle(&mut self, _msg: GetSocket, _ctx: &mut Self::Context) -> Self::Result {
        return self.clone();
    }
}

impl Actor for Socket {
    type Context = ws::WebsocketContext<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        self.id = Uuid::new_v4().to_string();
        self.addr = Some(ctx.address());
        let text = create_ws_msg(EventOrError::Event(Event::Start), &self.id).unwrap();
        ctx.text(text);
        println!("Start");
    }

    // fn
}
#[derive(Serialize, Deserialize, Clone)]
enum Event {
    Move,
    GameOver,
    Start,
    GetCode,
    ConnectWith,
    OppReady,
    Promote,
    PromoteReq,
}
#[derive(Clone, Serialize, Deserialize)]
enum EventError {
    ParseError,
    InvalidCode,
    RoomFull,
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
            Event::Promote => return String::from("Promote"),
            Event::PromoteReq => return String::from("PromoteReq"),
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
            "Promote" => return Ok(Event::Promote),
            "PromoteReq" => return Ok(Event::PromoteReq),
            _ => return Err(EventError::ParseError),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct WsMsg<E, M> {
    event: E,
    msg: M,
}

#[derive(Deserialize, Serialize, Clone)]
struct MoveMsg {
    room_code: String,
    i: u8,
    j: u8,
    k: u8,
    l: u8,
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for Socket {
    fn handle(&mut self, item: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match item {
            Ok(message) => match message {
                ws::Message::Text(text) => {
                    let text_string = text.to_string();
                    println!("{}", text_string);
                    let msg_struct = serde_json::from_str::<WsMsg<String, String>>(&text_string);

                    match msg_struct {
                        Ok(socket_msg) => {
                            let event = socket_msg.event;
                            let msg = socket_msg.msg;
                            match Event::from_string(&event) {
                                Ok(event) => match event {
                                    Event::Move => {
                                        let msg_struct =
                                            serde_json::from_str::<WsMsg<String, String>>(
                                                &text_string,
                                            );
                                        let mv = serde_json::from_str::<MoveMsg>(
                                            &msg_struct.unwrap().msg,
                                        );
                                        if let Ok(mv) = mv {
                                            let room_code = mv.room_code.parse::<u16>();
                                            if let Ok(room_code) = room_code {
                                                self.server.do_send(ServerCommands::Move(
                                                    self.addr.clone().unwrap(),
                                                    self.clone().id,
                                                    room_code,
                                                    (mv.i, mv.j),
                                                    (mv.k, mv.l),
                                                ))
                                            } else {
                                                let msg = create_ws_msg(
                                                    EventOrError::EventError(
                                                        EventError::ParseError,
                                                    ),
                                                    &"Invalid room code",
                                                )
                                                .unwrap();
                                                ctx.text(msg);
                                            }
                                        } else {
                                            let msg = create_ws_msg(
                                                EventOrError::EventError(EventError::ParseError),
                                                &"Invalid json",
                                            )
                                            .unwrap();
                                            ctx.text(msg);
                                        }
                                    }
                                    Event::GameOver => ctx.text("gameover"),
                                    Event::Start => ctx.text("starting"),
                                    Event::ConnectWith => {
                                        #[derive(Serialize, Deserialize)]
                                        struct ConnectWithMsg {
                                            room_code: String,
                                            name: String,
                                        }

                                        let connect_msg =
                                            serde_json::from_str::<ConnectWithMsg>(&msg);

                                        if let Ok(connect_msg) = connect_msg {
                                            let room_code =
                                                connect_msg.room_code.trim().parse::<u16>();
                                            if let Ok(code) = room_code {
                                                println!("passed code = {}", code);
                                                self.set_name(connect_msg.name);
                                                self.server.do_send(
                                                    ServerCommands::AddPlayerToRoom(
                                                        self.clone(),
                                                        code,
                                                    ),
                                                );
                                            } else {
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
                                        self.set_name(msg);
                                        self.server.do_send(ServerCommands::AddRoom(self.clone()));
                                    }
                                    Event::OppReady => {
                                        println!("{}", msg);
                                        let code = msg.trim().parse::<u16>();
                                        println!("Opp Ready");
                                        if let Ok(code) = code {
                                            self.server.do_send(ServerCommands::OppReady(
                                                self.clone(),
                                                code,
                                            ));
                                        } else {
                                            let msg = create_ws_msg(
                                                EventOrError::EventError(EventError::ParseError),
                                                &"Invalid Room Code",
                                            )
                                            .unwrap();
                                            ctx.text(msg);
                                        }
                                    }
                                    Event::Promote => {
                                        #[derive(Deserialize)]
                                        struct PromoteMsg {
                                            room_code: String,
                                            i: u8,
                                            j: u8,
                                            promote_to: String,
                                        }
                                        let promote_msg = serde_json::from_str::<PromoteMsg>(&msg);
                                        if let Ok(promote_msg) = promote_msg {
                                            let PromoteMsg {
                                                room_code,
                                                i,
                                                j,
                                                promote_to,
                                            } = promote_msg;
                                            let mut valid_promote = false;
                                            for k in ["H", "B", "Q", "R"] {
                                                if k == promote_to {
                                                    valid_promote = true;
                                                }
                                            }
                                            // ["H", "B", "Q", "R"].iter().collect();
                                            if let Ok(room_code) = room_code.parse::<u16>() {
                                                if (i == 0 || i == 7) && j < 8 && valid_promote {
                                                    self.server.do_send(ServerCommands::Promote(
                                                        room_code,
                                                        self.clone().id,
                                                        (i, j),
                                                        promote_to,
                                                    ))
                                                }
                                            }
                                        }
                                    }
                                    Event::PromoteReq => {
                                        // println!()
                                        // struct PromoteReqReq {
                                        //     room_code: String,
                                        // }
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
                        Err(err) => {
                            let res = create_ws_msg(
                                EventOrError::EventError(EventError::ParseError),
                                &err.to_string(),
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

#[derive(Serialize, Deserialize, Clone)]
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
