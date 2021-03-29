use actix::prelude::*;
use actix_cors::Cors;
use actix_web::{
    error, get,
    http::{header, StatusCode},
    post, web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder, Result,
};
use actix_web_actors::ws;
use std::{
    collections::{HashMap, VecDeque},
    sync::Mutex,
};
use tokio::sync::oneshot;

mod test;

#[derive(Default, Debug)]
struct RoomHost {
    answer_queue: VecDeque<oneshot::Sender<String>>,
}

#[derive(Message)]
#[rtype(result = "Result<String, ()>")]
struct Offer(String);

// impl Message for Offer {
//     // type Result = ResponseFuture<()>;
//     type Result = ResponseFuture<Result<(), ()>>;
//     //type Result = ResponseFuture<Result<String, ()>>;
// }

impl Actor for RoomHost {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        println!("Room opened");
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        println!("Room closed");
        // TODO: Remove from map
    }
}

impl Handler<Offer> for RoomHost {
    type Result = ResponseFuture<Result<String, ()>>;

    fn handle(&mut self, offer: Offer, ctx: &mut Self::Context) -> Self::Result {
        ctx.text(offer.0);
        let (tx, rx) = oneshot::channel();
        self.answer_queue.push_back(tx);
        Box::pin(async move { rx.await.map_err(|e| ()) }) // TODO
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for RoomHost {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(answer)) => {
                self.answer_queue.pop_front().unwrap().send(answer);
            }
            _ => (),
        }
    }
}

#[post("/join/{id}")]
async fn join(
    req: HttpRequest,
    offer: String,
    room_hosts: web::Data<Mutex<HashMap<String, Addr<RoomHost>>>>,
) -> impl Responder {
    let id = req.match_info().query("id");
    println!("Joining id: {:?}", id);
    println!("room_hosts = {:?}", room_hosts);
    let room_hosts_locked = room_hosts.lock().unwrap();
    if let Some(room_host) = room_hosts_locked.get(id) {
        let answer = room_host.send(Offer(offer)).await;

        match answer {
            Err(_) => HttpResponse::InternalServerError().body("no"), // TODO
            Ok(Err(_)) => HttpResponse::InternalServerError().body("no"), // TODO
            Ok(Ok(answer_text)) => HttpResponse::Ok().body(answer_text),
        }
    } else {
        HttpResponse::NotFound().body("Room not found")
    }
    //.map_err(|error| error::InternalError::new(error, StatusCode::INTERNAL_SERVER_ERROR))?;
    //answer.map_err(|error| error::InternalError::new(error, StatusCode::INTERNAL_SERVER_ERROR))
}

#[get("/host")]
async fn host(
    req: HttpRequest,
    stream: web::Payload,
    room_hosts: web::Data<Mutex<HashMap<String, Addr<RoomHost>>>>,
) -> Result<HttpResponse, Error> {
    let (addr, resp) = ws::start_with_addr(RoomHost::default(), &req, stream)?;
    //let id = nanoid::simple(); // TODO
    let id = "1234".to_string();
    println!("Hosting id: {:?}", id);
    room_hosts.lock().unwrap().insert(id, addr);
    println!("room_hosts = {:?}", room_hosts);
    Ok(resp)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let room_hosts = web::Data::new(Mutex::new(HashMap::<String, Addr<RoomHost>>::new()));
    HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header(),
            )
            .app_data(room_hosts.clone())
            .service(join)
            .service(host)
    })
    .bind("192.168.1.9:8080")?
    .run()
    .await
}
