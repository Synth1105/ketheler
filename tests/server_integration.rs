use ketheler::server::{
    self, CallError, CallRef, Response, ResponseKind, Server, TerminateReason,
};

struct Counter;

enum CallMsg {
    Get,
    Add(u64),
}

enum CastMsg {
    Inc,
}

impl Server for Counter {
    type State = u64;
    type Call = CallMsg;
    type Cast = CastMsg;
    type Info = ();
    type Reply = u64;

    fn init() -> Self::State {
        0
    }

    fn handle_call(
        call: Self::Call,
        _from: CallRef<Self::Reply>,
        state: Self::State,
    ) -> Response<Self::Reply, Self::State> {
        match call {
            CallMsg::Get => Response::Reply(state, state, Some(ResponseKind::Call)),
            CallMsg::Add(value) => {
                let next = state + value;
                Response::Reply(next, next, Some(ResponseKind::Call))
            }
        }
    }

    fn handle_cast(cast: Self::Cast, state: Self::State) -> Response<Self::Reply, Self::State> {
        match cast {
            CastMsg::Inc => Response::NoReply(state + 1, Some(ResponseKind::Cast)),
        }
    }
}

#[test]
fn server_call_and_cast_work() {
    let handle = server::start_link::<Counter>();
    handle.cast(CastMsg::Inc).unwrap();
    let value = handle.call(CallMsg::Add(2)).unwrap();
    assert_eq!(value, 3);
    let value = handle.call(CallMsg::Get).unwrap();
    assert_eq!(value, 3);
    handle.stop(TerminateReason::Normal).unwrap();
}

#[test]
fn server_stop_makes_calls_fail() {
    let handle = server::start_link::<Counter>();
    handle.stop(TerminateReason::Shutdown).unwrap();
    let result = handle.call(CallMsg::Get);
    assert_eq!(result, Err(CallError::ServerDown));
}
