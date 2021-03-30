use actix::prelude::*;
use actix_cors::Cors;
use actix_web::{
    get, post, web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder, Result,
};
use actix_web_actors::ws;
use std::collections::{HashMap, VecDeque};
use tokio::sync::oneshot;

mod test;

#[derive(Default, Debug)]
struct RoomRegistry {
    rooms: HashMap<String, Addr<RoomHost>>,
}

#[derive(Message)]
#[rtype(result = "()")]
struct HostConnect(String, Addr<RoomHost>);

#[derive(Message)]
#[rtype(result = "()")]
struct HostDisconnect(String);

#[derive(Message)]
#[rtype(result = "Option<Addr<RoomHost>>")]
struct GetHost(String);

impl Actor for RoomRegistry {
    type Context = Context<Self>;
}

impl Handler<HostConnect> for RoomRegistry {
    type Result = ();
    fn handle(&mut self, msg: HostConnect, ctx: &mut Self::Context) {
        let HostConnect(id, host_address) = msg;
        self.rooms.insert(id, host_address);
    }
}

impl Handler<HostDisconnect> for RoomRegistry {
    type Result = ();
    fn handle(&mut self, msg: HostDisconnect, ctx: &mut Self::Context) {
        let HostDisconnect(id) = msg;
        self.rooms.remove(&id);
    }
}

impl Handler<GetHost> for RoomRegistry {
    type Result = Option<Addr<RoomHost>>;
    fn handle(&mut self, msg: GetHost, ctx: &mut Self::Context) -> Self::Result {
        let GetHost(id) = msg;
        self.rooms.get(&id).cloned()
    }
}

#[derive(Debug)]
struct RoomHost {
    answer_queue: VecDeque<oneshot::Sender<String>>,
    id: String,
    registry_address: Addr<RoomRegistry>,
}

#[derive(Message)]
#[rtype(result = "Result<String, ()>")]
struct Offer(String);

impl RoomHost {
    fn new(registry_address: Addr<RoomRegistry>) -> RoomHost {
        RoomHost {
            answer_queue: Default::default(),
            id: nanoid::simple(),
            registry_address,
        }
    }
}

impl Actor for RoomHost {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        println!("Registering new room...");
        self.registry_address
            .do_send(HostConnect(self.id.clone(), ctx.address()));
        // TODO: error handling
        println!("Room opened");
        ctx.text(self.id.clone());
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        println!("Deregistering room...");
        self.registry_address
            .do_send(HostDisconnect(self.id.clone()));
        println!("Room closed");
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
    room_registry_address: web::Data<Addr<RoomRegistry>>,
) -> impl Responder {
    let id = req.match_info().query("id").to_string();
    match room_registry_address.send(GetHost(id)).await {
        Ok(Some(room_host)) => {
            let answer = room_host.send(Offer(offer)).await;

            match answer {
                Err(_) => HttpResponse::InternalServerError().body(""), // TODO
                Ok(Err(_)) => HttpResponse::InternalServerError().body(""), // TODO
                Ok(Ok(answer_text)) => HttpResponse::Ok().body(answer_text),
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Room with given id not found"),
        Err(_) => HttpResponse::InternalServerError().body(""), // TODO
    }
}

#[get("/host")]
async fn host(
    req: HttpRequest,
    stream: web::Payload,
    room_registry_address: web::Data<Addr<RoomRegistry>>,
) -> Result<HttpResponse, Error> {
    println!("Starting new host actor...");
    let resp = ws::start(
        RoomHost::new(room_registry_address.get_ref().clone()),
        &req,
        stream,
    )?;
    Ok(resp)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let room_registry_address = RoomRegistry::default().start();
    HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header(),
            )
            .data(room_registry_address.clone())
            .service(join)
            .service(host)
    })
    .bind("192.168.1.9:8080")?
    .run()
    .await
}
