use ketheler::server::{self, CallRef, CallResponse, Server};
use std::fmt;

#[derive(Clone, Copy)]
struct Echo(u32);

impl fmt::Display for Echo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Server for Echo {
    type State = Self;
    type Call = ();
    type Cast = ();
    type Info = ();
    type Reply = ();

    fn init() -> Self::State {
        Echo(0)
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
    server::debug(Echo(7));
}
