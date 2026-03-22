use ketheler::server::{self, CallError, CallRef, CallResponse, CastResponse, Server, TerminateReason};

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
    ) -> CallResponse<Self::Reply, Self::State> {
        match call {
            CallMsg::Get => CallResponse::Reply(state, state),
            CallMsg::Add(value) => {
                let next = state + value;
                CallResponse::Reply(next, next)
            }
        }
    }

    fn handle_cast(cast: Self::Cast, state: Self::State) -> CastResponse<Self::State> {
        match cast {
            CastMsg::Inc => CastResponse::NoReply(state + 1),
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
