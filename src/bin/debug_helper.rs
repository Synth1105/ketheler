use ketheler::server::{self, CallRef, Response, ResponseKind, Server};

#[derive(Clone, Copy, Debug)]
struct Echo(u32);

impl Server for Echo {
    type State = Self;
    type Call = ();
    type Cast = ();
    type Info = ();
    type Reply = ();

    fn init() -> Self::State {
        Echo(7)
    }

    fn handle_call(
        _call: Self::Call,
        _from: CallRef<Self::Reply>,
        state: Self::State,
    ) -> Response<Self::Reply, Self::State> {
        Response::Reply((), state, Some(ResponseKind::Call))
    }
}

fn main() {
    let handle = server::start_link::<Echo>();
    server::debug(&handle).unwrap();
}
