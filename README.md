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

## Debugging And Status (OTP-style)

`ServerHandle::status()` requests OTP-style status data from a running server.
By default, `handle_status` uses the `Debug` representation of the state.

```rust
use ketheler::server;

let handle = server::start_link::<Echo>();
let status = handle.status().unwrap();
println!("{}", status.state);
```

The `debug` helper prints the status state string:

```rust
use ketheler::server;

let handle = server::start_link::<Echo>();
server::debug(&handle).unwrap();
```

If you want to format a state value directly (not a running server), use
`debug_state` or `debug_state_with`:

```rust
use ketheler::server;

server::debug_state::<Echo>(Echo(7));
server::debug_state_with(Echo, Echo(7));
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
