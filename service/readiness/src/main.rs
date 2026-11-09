//! Service readiness and backpressure.
//!
//! Before a pipeline hands a request to a service, it waits until the service
//! is *ready*. This example shows the three outcomes of a readiness check:
//!
//! * **Ready.** The request is passed on.
//! * **Pending.** The service is busy. The caller waits, and the request stays
//!   with the caller instead of piling up inside the service. This is
//!   backpressure. See [`limit::ConcurrencyLimit`].
//! * **Error.** The service can't accept requests at all. The caller gets the
//!   error and the request is never passed on. See [`gate::Gate`].
//!
//! Run it with `cargo run -p service-readiness` and follow the timestamps in
//! the output.

use std::cell::Cell;
use std::convert::Infallible;
use std::fmt;
use std::time::{Duration, Instant};

use ntex::service::{Pipeline, Service, fn_service, service};
use ntex::time::sleep;
use ntex::util::join_all;

/// Prints a line prefixed with the elapsed time and the component name.
macro_rules! log {
    ($who:expr, $($arg:tt)*) => {
        $crate::print_line($who, format_args!($($arg)*))
    };
}

mod gate;
mod limit;

use gate::{Gate, GateHandle};
use limit::ConcurrencyLimit;

thread_local! {
    static START: Cell<Option<Instant>> = const { Cell::new(None) };
}

/// Writes one line of console output, such as
/// `[  100ms] limit  | slot taken: 2/2 in flight`.
///
/// The clock starts at the first line printed on the current thread.
fn print_line(who: &str, args: fmt::Arguments<'_>) {
    let start = START.with(|start| {
        let now = start.get().unwrap_or_else(Instant::now);
        start.set(Some(now));
        now
    });
    let elapsed = start.elapsed().as_millis();
    println!("[{elapsed:>5}ms] {who:<6} | {args}");
}

/// A unit of work: an id and how long it takes.
#[derive(Debug, Clone, Copy)]
pub struct Job {
    pub id: usize,
    pub duration: Duration,
}

impl Job {
    pub fn new(id: usize, millis: u64) -> Self {
        Self {
            id,
            duration: Duration::from_millis(millis),
        }
    }
}

/// The service doing the actual work. It sleeps for the job's duration and
/// returns the job id.
///
/// It is always ready, so it never limits anything on its own. The wrappers
/// in [`limit`] and [`gate`] add the readiness behavior.
pub fn worker() -> impl Service<(), Job, Res = usize, Error = Infallible> {
    fn_service(async |job: Job| {
        log!("worker", "job {} started", job.id);
        sleep(job.duration).await;
        log!("worker", "job {} finished", job.id);
        Ok(job.id)
    })
}

/// Sends five jobs at once to a chain whose last service allows only two
/// calls at a time.
///
/// The first service in the chain, `front`, is always ready. It still
/// accepts only two jobs at first: a chain is ready only when every service
/// in it is ready, so the full limit at the end holds back new work at the
/// front. Every time a job finishes, one more is accepted.
async fn backpressure() {
    let front = fn_service(async |job: Job| {
        log!("front", "accepted job {}", job.id);
        Ok::<_, Infallible>(job)
    });
    let pipeline = Pipeline::new(
        (),
        service(front).and_then(ConcurrencyLimit::new(2, worker())),
    );

    let jobs = [
        Job::new(1, 300),
        Job::new(2, 100),
        Job::new(3, 200),
        Job::new(4, 100),
        Job::new(5, 100),
    ];
    let calls = jobs.map(|job| {
        log!("client", "sending job {}", job.id);
        pipeline.call(job)
    });

    for result in join_all(calls).await {
        log!("client", "result: {result:?}");
    }
}

/// Calls a gated service while the gate is open, closed, and open again.
///
/// While the gate is closed, the readiness check fails, the caller receives
/// the error, and the worker never sees the request.
async fn failing_readiness() {
    let gate = GateHandle::new();
    let pipeline = Pipeline::new((), Gate::new(gate.clone(), worker()));

    log!("client", "sending job 10");
    log!(
        "client",
        "result: {:?}",
        pipeline.call(Job::new(10, 50)).await
    );

    gate.close();
    log!("client", "sending job 11");
    log!(
        "client",
        "result: {:?}",
        pipeline.call(Job::new(11, 50)).await
    );

    gate.open();
    log!("client", "sending job 12");
    log!(
        "client",
        "result: {:?}",
        pipeline.call(Job::new(12, 50)).await
    );
}

#[ntex::main]
async fn main() {
    println!("== Backpressure: at most 2 jobs at a time ==");
    backpressure().await;

    println!();
    println!("== Failing readiness: a gate that can be closed ==");
    failing_readiness().await;
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use std::task::Poll;

    use ntex::channel::oneshot;
    use ntex::util::lazy;

    use super::gate::GateError;
    use super::*;

    /// Returns a worker that counts its calls and records the peak number of
    /// calls running at the same time.
    fn counting_worker(
        calls: Rc<Cell<usize>>,
        peak: Rc<Cell<usize>>,
    ) -> impl Service<(), Job, Res = usize, Error = Infallible> {
        let running = Rc::new(Cell::new(0));
        fn_service(async move |job: Job| {
            calls.set(calls.get() + 1);
            running.set(running.get() + 1);
            peak.set(peak.get().max(running.get()));
            sleep(job.duration).await;
            running.set(running.get() - 1);
            Ok(job.id)
        })
    }

    /// STATEMENT: `ConcurrencyLimit` never lets more than `max` calls run at
    /// once, and every queued call still completes.
    ///
    /// HOW:
    /// - Wrap a worker that records its peak concurrency in a limit of 2.
    /// - Send six 20 ms jobs at once and wait for all of them.
    /// - Confirm all six succeeded in order and the peak was exactly 2.
    #[ntex::test]
    async fn limit_caps_concurrent_calls() {
        let calls = Rc::new(Cell::new(0));
        let peak = Rc::new(Cell::new(0));
        let pipeline = Pipeline::new(
            (),
            ConcurrencyLimit::new(2, counting_worker(calls.clone(), peak.clone())),
        );

        let results = join_all((0..6).map(|id| pipeline.call(Job::new(id, 20)))).await;

        let ids: Vec<_> = results.into_iter().map(Result::unwrap).collect();
        assert_eq!(ids, vec![0, 1, 2, 3, 4, 5]);
        assert_eq!(calls.get(), 6);
        assert_eq!(peak.get(), 2);
    }

    /// STATEMENT: Taking the last free slot invalidates the pipeline's cached
    /// readiness, and freeing it makes the pipeline ready again.
    ///
    /// HOW:
    /// - Build a pipeline with a limit of 1 around a worker that signals when
    ///   it starts and then waits for a release signal, and confirm the
    ///   pipeline is ready.
    /// - Spawn one call and wait until the worker reports it started.
    /// - Confirm `poll_ready()` is now pending, then release the worker, wait
    ///   for the call to end and confirm `poll_ready()` succeeds.
    #[ntex::test]
    async fn full_limit_invalidates_readiness() {
        let (started_tx, started_rx) = oneshot::channel();
        let (release_tx, release_rx) = oneshot::channel::<()>();
        let signals = RefCell::new(Some((started_tx, release_rx)));
        let worker = fn_service(move |job: Job| {
            let (started, release) = signals.borrow_mut().take().expect("a single call");
            async move {
                let _ = started.send(());
                let _ = release.await;
                Ok::<_, Infallible>(job.id)
            }
        });

        let pipeline = Pipeline::new((), ConcurrencyLimit::new(1, worker));
        assert_eq!(
            lazy(|cx| pipeline.poll_ready(cx)).await,
            Poll::Ready(Ok(()))
        );

        let handle = ntex::rt::spawn(pipeline.call_static(Job::new(1, 0)));
        started_rx.await.unwrap();
        assert_eq!(lazy(|cx| pipeline.poll_ready(cx)).await, Poll::Pending);

        release_tx.send(()).unwrap();
        assert_eq!(handle.await.unwrap(), Ok(1));
        assert_eq!(
            lazy(|cx| pipeline.poll_ready(cx)).await,
            Poll::Ready(Ok(()))
        );
    }

    /// STATEMENT: Closing a `Gate` after a successful call makes the next
    /// readiness check fail, and the request never reaches the wrapped
    /// service until the gate is opened again.
    ///
    /// HOW:
    /// - Call through an open gate, so the pipeline caches readiness.
    /// - Close the gate, call again, and confirm `GateError::Closed` with the
    ///   worker's call count unchanged.
    /// - Open the gate and confirm the next call reaches the worker.
    #[ntex::test]
    async fn closed_gate_fails_readiness() {
        let calls = Rc::new(Cell::new(0));
        let gate = GateHandle::new();
        let pipeline = Pipeline::new(
            (),
            Gate::new(gate.clone(), counting_worker(calls.clone(), Rc::default())),
        );

        assert_eq!(pipeline.call(Job::new(1, 1)).await, Ok(1));
        assert_eq!(calls.get(), 1);

        gate.close();
        assert_eq!(pipeline.call(Job::new(2, 1)).await, Err(GateError::Closed));
        assert_eq!(calls.get(), 1);

        gate.open();
        assert_eq!(pipeline.call(Job::new(3, 1)).await, Ok(3));
        assert_eq!(calls.get(), 2);
    }
}
