use actix::prelude::*;

#[derive(Message)]
#[rtype(result = "Result<(), ()>")]
struct Msg;

struct MyActor;

impl Actor for MyActor {
    type Context = Context<Self>;
}

impl Handler<Msg> for MyActor {
    type Result = ResponseFuture<Result<(), ()>>;

    fn handle(&mut self, _: Msg, _: &mut Context<Self>) -> Self::Result {
        Box::pin(async move {
            // Some async computation
            Ok(())
        })
    }
}
