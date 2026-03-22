//! A small green-thread scheduler built on top of `may`.
//!
//! This module mirrors the Elixir-style scheduler pattern:
//! workers announce readiness, the scheduler hands out work, and results are
//! collected and sorted by input.
//!
//! # Example
//! ```no_run
//! use ketheler::scheduler;
//!
//! fn fib(n: u64) -> u64 {
//!     match n {
//!         0 => 0,
//!         1 => 1,
//!         _ => fib(n - 1) + fib(n - 2),
//!     }
//! }
//!
//! let inputs = vec![35_u64, 36, 37, 38];
//! let results = scheduler::run(4, fib, inputs);
//! # let _ = results;
//! ```

use may::coroutine;
use may::sync::mpsc;
use std::collections::VecDeque;
use std::sync::Arc;

enum SchedulerMsg<T, R> {
    Ready(usize),
    Answer { input: T, result: R },
}

enum WorkerMsg<T> {
    Work(T),
    Shutdown,
}

/// Runs a pool of green-thread workers over a queue of inputs.
///
/// The scheduler hands out one item at a time to a ready worker and collects
/// results. The final output is sorted by the input value.
pub fn run<T, R, F>(num_processes: u16, func: F, to_calculate: Vec<T>) -> Vec<(T, R)>
where
    F: Fn(T) -> R + Send + Sync + 'static,
    T: Ord + Send + Clone + 'static,
    R: Send + 'static,
{
    let workers = if num_processes == 0 {
        1
    } else {
        num_processes as usize
    };
    may::config().set_workers(workers);

    let (in_tx, in_rx) = mpsc::channel::<SchedulerMsg<T, R>>();
    let func = Arc::new(func);
    let mut worker_chans: Vec<mpsc::Sender<WorkerMsg<T>>> = Vec::with_capacity(workers);

    for id in 0..workers {
        let inbox = in_tx.clone();
        let func = Arc::clone(&func);
        let (tx, rx) = mpsc::channel::<WorkerMsg<T>>();
        worker_chans.push(tx);
        unsafe {
            coroutine::spawn(move || worker_loop(id, func, inbox, rx));
        }
    }

    let mut queue: VecDeque<T> = to_calculate.into();
    let mut alive = vec![true; workers];
    let mut alive_count = workers;
    let mut results: Vec<(T, R)> = Vec::new();

    while alive_count > 0 {
        match in_rx.recv() {
            Ok(SchedulerMsg::Ready(id)) => {
                if let Some(next) = queue.pop_front() {
                    let _ = worker_chans[id].send(WorkerMsg::Work(next));
                } else {
                    let _ = worker_chans[id].send(WorkerMsg::Shutdown);
                    if alive[id] {
                        alive[id] = false;
                        alive_count -= 1;
                    }
                }
            }
            Ok(SchedulerMsg::Answer { input, result }) => {
                results.push((input, result));
            }
            Err(_) => break,
        }
    }

    results.sort_by(|a, b| a.0.cmp(&b.0));
    results
}

fn worker_loop<T, R, F>(
    id: usize,
    func: Arc<F>,
    inbox: mpsc::Sender<SchedulerMsg<T, R>>,
    cmd: mpsc::Receiver<WorkerMsg<T>>,
) where
    F: Fn(T) -> R + Send + Sync + 'static,
    T: Send + Clone + 'static,
    R: Send + 'static,
{
    loop {
        let _ = inbox.send(SchedulerMsg::Ready(id));
        match cmd.recv() {
            Ok(WorkerMsg::Work(input)) => {
                let result = (func)(input.clone());
                let _ = inbox.send(SchedulerMsg::Answer { input, result });
            }
            Ok(WorkerMsg::Shutdown) | Err(_) => break,
        }
    }
}
