use actix_cors::Cors;
use actix_web::{get, http, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use actix_web_actors::ws;

mod socket;
use socket::Socket;

#[get("/")]
async fn test() -> impl Responder {
    HttpResponse::Ok().body("Hello")
}

#[get("/ws")]
async fn get_ws(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, actix_web::Error> {
    let resp = ws::start(
        Socket {
            id: String::from("0"),
        },
        &req,
        stream,
    );
    println!("{:?}", resp);
    resp
}

#[actix_web::main]
async fn main() -> Result<(), std::io::Error> {
    // let server = server::ChessServer::new().start();
    let server_addr = "127.0.0.1";
    let server_port = 8080;
    let app = HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin("http://localhost:5173")
            .allowed_origin("http://localhost:8080")
            .allow_any_origin()
            .allowed_methods(vec!["GET", "POST", "WS"])
            .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT])
            .allowed_header(http::header::CONTENT_TYPE)
            .max_age(3600);
        App::new().wrap(cors).service(test).service(get_ws)
    })
    .bind((server_addr, server_port))?
    .run();
    println!("Server running at http://{server_addr}:{server_port}");
    app.await
}
