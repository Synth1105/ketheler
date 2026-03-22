# Ketheler

Ketheler is an OTP-inspired concurrency toolkit for Rust. It provides:

- `server`: a GenServer-style abstraction for stateful processes with `call`, `cast`, and `info` messages
- `scheduler`: a small green-thread work scheduler built on top of `may`

## Install

Add to your `Cargo.toml`:

```toml
[dependencies]
ketheler = "0.2.0"
```

## GenServer-Style API

```rust
use ketheler::server::{self, CallResponse, CastResponse, Server};

struct Counter;

enum Call {
    Get,
    Add(u64),
}

enum Cast {
    Inc,
}

impl Server for Counter {
    type State = u64;
    type Call = Call;
    type Cast = Cast;
    type Info = ();
    type Reply = u64;

    fn init() -> Self::State {
        0
    }

    fn handle_call(
        call: Self::Call,
        _from: server::CallRef<Self::Reply>,
        state: Self::State,
    ) -> CallResponse<Self::Reply, Self::State> {
        match call {
            Call::Get => CallResponse::Reply(state, state),
            Call::Add(delta) => {
                let next = state + delta;
                CallResponse::Reply(next, next)
            }
        }
    }

    fn handle_cast(
        cast: Self::Cast,
        state: Self::State,
    ) -> CastResponse<Self::State> {
        match cast {
            Cast::Inc => CastResponse::NoReply(state + 1),
        }
    }
}

let handle = server::start_link::<Counter>();
handle.cast(Cast::Inc).unwrap();
let value = handle.call(Call::Get).unwrap();
assert_eq!(value, 1);
```

## Debugging State

The `debug` helper prints the output of `Server::handle_debug` for a state value.

```rust
use ketheler::server;

// `Echo` implements `Server` with `State = Echo`.
server::debug::<Echo>(Echo(7));
```

## Scheduler

```rust
use ketheler::scheduler;

fn fib(n: u64) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => fib(n - 1) + fib(n - 2),
    }
}

let inputs = vec![35_u64, 36, 37, 38];
let results = scheduler::run(4, fib, inputs);
```

## Tests

```bash
cargo test
```
