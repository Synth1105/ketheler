use ketheler::server::{self, CallRef, CallResponse, Server};

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
    ) -> CallResponse<Self::Reply, Self::State> {
        CallResponse::Reply((), state)
    }
}

fn main() {
    let handle = server::start_link::<Echo>();
    server::debug(&handle).unwrap();
}
