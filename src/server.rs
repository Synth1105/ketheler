//! OTP-style GenServer implementation built on top of `may`.
//!
//! The API mirrors the Erlang/Elixir idea of a long-lived process that owns
//! state and receives `call`, `cast`, or `info` messages.
//!
//! # Example
//! ```no_run
//! use ketheler::server::{self, CallResponse, CastResponse, Server};
//!
//! struct Counter;
//!
//! enum Call {
//!     Get,
//!     Add(u64),
//! }
//!
//! enum Cast {
//!     Inc,
//! }
//!
//! impl Server for Counter {
//!     type State = u64;
//!     type Call = Call;
//!     type Cast = Cast;
//!     type Info = ();
//!     type Reply = u64;
//!
//!     fn init() -> Self::State {
//!         0
//!     }
//!
//!     fn handle_call(
//!         call: Self::Call,
//!         _from: server::CallRef<Self::Reply>,
//!         state: Self::State,
//!     ) -> CallResponse<Self::Reply, Self::State> {
//!         match call {
//!             Call::Get => CallResponse::Reply(state, state),
//!             Call::Add(delta) => {
//!                 let next = state + delta;
//!                 CallResponse::Reply(next, next)
//!             }
//!         }
//!     }
//!
//!     fn handle_cast(
//!         cast: Self::Cast,
//!         state: Self::State,
//!     ) -> CastResponse<Self::State> {
//!         match cast {
//!             Cast::Inc => CastResponse::NoReply(state + 1),
//!         }
//!     }
//! }
//!
//! let handle = server::start_link::<Counter>();
//! handle.cast(Cast::Inc).unwrap();
//! let value = handle.call(Call::Get).unwrap();
//! assert_eq!(value, 1);
//! ```

use may::coroutine;
use may::sync::mpsc;
use std::fmt::Debug;

/// Reason given when a server terminates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminateReason {
    /// Normal shutdown.
    Normal,
    /// Shutdown requested by a supervisor or peer.
    Shutdown,
    /// Error-driven termination.
    Error(String),
}

/// Reply options for synchronous calls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallResponse<R, S> {
    /// Reply immediately and continue with a new state.
    Reply(R, S),
    /// Do not reply immediately; the handler is responsible for replying later.
    NoReply(S),
    /// Stop the server, optionally replying first.
    Stop(TerminateReason, S, Option<R>),
}

/// Response options for asynchronous casts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CastResponse<S> {
    /// Continue without a reply.
    NoReply(S),
    /// Stop the server.
    Stop(TerminateReason, S),
}

/// Response options for generic info messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InfoResponse<S> {
    /// Continue without a reply.
    NoReply(S),
    /// Stop the server.
    Stop(TerminateReason, S),
}

/// Handle used by a server to reply to a call later.
#[derive(Debug)]
pub struct CallRef<R> {
    reply: mpsc::Sender<R>,
}

impl<R> CallRef<R> {
    /// Reply to the caller.
    pub fn reply(&self, reply: R) -> Result<(), R> {
        self.reply.send(reply).map_err(|err| err.0)
    }
}

impl<R> Clone for CallRef<R> {
    fn clone(&self) -> Self {
        Self {
            reply: self.reply.clone(),
        }
    }
}

/// Trait implemented by OTP-style servers.
pub trait Server {
    /// Internal state type.
    type State: Send + 'static;
    /// Synchronous call message type.
    type Call: Send + 'static;
    /// Asynchronous cast message type.
    type Cast: Send + 'static;
    /// Generic info message type.
    type Info: Send + 'static;
    /// Reply type for calls.
    type Reply: Send + 'static;

    /// Initialize the server state.
    fn init() -> Self::State;

    /// Handle a synchronous call.
    fn handle_call(
        call: Self::Call,
        from: CallRef<Self::Reply>,
        state: Self::State,
    ) -> CallResponse<Self::Reply, Self::State>;

    /// Handle an asynchronous cast.
    fn handle_cast(cast: Self::Cast, state: Self::State) -> CastResponse<Self::State> {
        let _ = cast;
        CastResponse::NoReply(state)
    }

    /// Handle an info message (anything not call/cast).
    fn handle_other(info: Self::Info, state: Self::State) -> InfoResponse<Self::State> {
        let _ = info;
        InfoResponse::NoReply(state)
    }

    /// Called when the server stops for any reason.
    fn handle_halt(reason: TerminateReason, state: Self::State) {
        let _ = (reason, state);
    }

    /// Handle a status request (OTP-style).
    fn handle_info(state: &Self::State) -> String
    where
        <Self as Server>::State: std::fmt::Debug,
    {
        format!("{:?}", state)
    }
}


enum ServerMsg<S: Server> {
    Call {
        msg: S::Call,
        from: CallRef<S::Reply>,
    },
    Cast(S::Cast),
    Info(S::Info),
    Status {
        reply: mpsc::Sender<String>,
    },
    Stop(TerminateReason),
}

/// Errors when issuing calls to a server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallError {
    /// The server has already stopped.
    ServerDown,
}

/// Errors when sending cast/info/stop messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendError {
    /// The server has already stopped.
    ServerDown,
}

/// Snapshot of a server's status (OTP-style).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerStatus {
    /// Fully-qualified type name of the server.
    pub server: &'static str,
    /// Server-provided status string.
    pub state: String,
}

/// A handle to a running server.
#[derive(Debug)]
pub struct ServerHandle<S: Server> {
    tx: mpsc::Sender<ServerMsg<S>>,
}

impl<S: Server> ServerHandle<S> {
    /// Send a synchronous call and wait for the reply.
    pub fn call(&self, msg: S::Call) -> Result<S::Reply, CallError> {
        let (tx, rx) = mpsc::channel::<S::Reply>();
        let from = CallRef { reply: tx };
        self.tx
            .send(ServerMsg::Call { msg, from })
            .map_err(|_| CallError::ServerDown)?;
        rx.recv().map_err(|_| CallError::ServerDown)
    }

    /// Send an asynchronous cast.
    pub fn cast(&self, msg: S::Cast) -> Result<(), SendError> {
        self.tx
            .send(ServerMsg::Cast(msg))
            .map_err(|_| SendError::ServerDown)
    }

    /// Send an info message.
    pub fn other(&self, msg: S::Info) -> Result<(), SendError> {
        self.tx
            .send(ServerMsg::Info(msg))
            .map_err(|_| SendError::ServerDown)
    }

    /// Stop the server.
    pub fn stop(&self, reason: TerminateReason) -> Result<(), SendError> {
        self.tx
            .send(ServerMsg::Stop(reason))
            .map_err(|_| SendError::ServerDown)
    }

    /// Request OTP-style status information from the server.
    pub fn info(&self) -> Result<ServerStatus, CallError> {
        let (tx, rx) = mpsc::channel::<String>();
        self.tx
            .send(ServerMsg::Status { reply: tx })
            .map_err(|_| CallError::ServerDown)?;
        let state = rx.recv().map_err(|_| CallError::ServerDown)?;
        Ok(ServerStatus {
            server: std::any::type_name::<S>(),
            state,
        })
    }
}

/// Start a linked server process.
pub fn start_link<S: Server + 'static>() -> ServerHandle<S> where <S as Server>::State: Debug {
    let (tx, rx) = mpsc::channel::<ServerMsg<S>>();
    unsafe {
        coroutine::spawn(move || server_loop::<S>(rx));
    }
    ServerHandle { tx }
}

impl<S: Server> Clone for ServerHandle<S> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}

fn server_loop<S: Server>(rx: mpsc::Receiver<ServerMsg<S>>) where <S as Server>::State: Debug  {
    let mut state = S::init();

    loop {
        let msg = match rx.recv() {
            Ok(msg) => msg,
            Err(_) => break,
        };

        match msg {
            ServerMsg::Call { msg, from } => {
                let from_for_handler = from.clone();
                let response = S::handle_call(msg, from_for_handler, state);
                match response {
                    CallResponse::Reply(reply, new_state) => {
                        let _ = from.reply(reply);
                        state = new_state;
                    }
                    CallResponse::NoReply(new_state) => {
                        state = new_state;
                    }
                    CallResponse::Stop(reason, new_state, maybe_reply) => {
                        if let Some(reply) = maybe_reply {
                            let _ = from.reply(reply);
                        }
                        S::handle_halt(reason, new_state);
                        break;
                    }
                }
            }
            ServerMsg::Cast(msg) => {
                let response = S::handle_cast(msg, state);
                match response {
                    CastResponse::NoReply(new_state) => {
                        state = new_state;
                    }
                    CastResponse::Stop(reason, new_state) => {
                        S::handle_halt(reason, new_state);
                        break;
                    }
                }
            }
            ServerMsg::Info(msg) => {
                let response = S::handle_other(msg, state);
                match response {
                    InfoResponse::NoReply(new_state) => {
                        state = new_state;
                    }
                    InfoResponse::Stop(reason, new_state) => {
                        S::handle_halt(reason, new_state);
                        break;
                    }
                }
            }
            ServerMsg::Status { reply } => {
                let status = S::handle_info(&state);
                let _ = reply.send(status);
            }
            ServerMsg::Stop(reason) => {
                S::handle_halt(reason, state);
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    struct Counter;

    #[derive(Debug)]
    enum CallMsg {
        Get,
        Add(u64),
    }

    #[derive(Debug)]
    enum CastMsg {
        Inc,
        Add(u64),
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
                CastMsg::Add(value) => CastResponse::NoReply(state + value),
            }
        }
    }

    #[test]
    fn call_and_cast_update_state() {
        let handle = start_link::<Counter>();
        handle.cast(CastMsg::Inc).unwrap();
        handle.cast(CastMsg::Add(4)).unwrap();
        let value = handle.call(CallMsg::Add(2)).unwrap();
        assert_eq!(value, 7);
        let value = handle.call(CallMsg::Get).unwrap();
        assert_eq!(value, 7);
    }

    struct Deferred;

    enum DeferredCall {
        Wait,
    }

    enum DeferredCast {
        ReplyNow(u32),
    }

    impl Server for Deferred {
        type State = Option<CallRef<u32>>;
        type Call = DeferredCall;
        type Cast = DeferredCast;
        type Info = ();
        type Reply = u32;

        fn init() -> Self::State {
            None
        }

        fn handle_call(
            _call: Self::Call,
            from: CallRef<Self::Reply>,
            _state: Self::State,
        ) -> CallResponse<Self::Reply, Self::State> {
            CallResponse::NoReply(Some(from))
        }

        fn handle_cast(cast: Self::Cast, state: Self::State) -> CastResponse<Self::State> {
            match cast {
                DeferredCast::ReplyNow(value) => {
                    if let Some(from) = state {
                        let _ = from.reply(value);
                    }
                    CastResponse::NoReply(None)
                }
            }
        }
    }    

    #[test]
    fn reply_later_with_callref() {
        let handle = start_link::<Deferred>();
        let handle_for_cast = handle.clone();

        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(10));
            handle_for_cast
                .cast(DeferredCast::ReplyNow(7))
                .unwrap();
        });

        let reply = handle.call(DeferredCall::Wait).unwrap();
        assert_eq!(reply, 7);
    }

    #[test]
    fn stop_makes_future_calls_fail() {
        let handle = start_link::<Counter>();
        handle.stop(TerminateReason::Shutdown).unwrap();
        std::thread::sleep(Duration::from_millis(10));
        let result = handle.call(CallMsg::Get);
        assert_eq!(result, Err(CallError::ServerDown));
    }

}

/// Prints OTP-style status info from a running server.
pub fn debug<S>(handle: &ServerHandle<S>) -> Result<(), CallError>
where
    S: Server,
{
    let status = handle.info()?;
    println!("{}", status.state);
    Ok(())
}


