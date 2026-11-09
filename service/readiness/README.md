# Service Readiness and Backpressure

Every ntex service answers two questions: *can you take a request now?*
(`ready`) and *please handle this request* (`call`). A pipeline always asks
the first question before it asks the second. A service that is busy or
unavailable can then say so before any work reaches it.

A readiness check has three possible outcomes:

- **Ready.** The pipeline passes the request on.
- **Pending.** The service is busy. The caller waits, and the request stays
  with the caller instead of piling up inside the service. This is
  backpressure.
- **Error.** The service can't accept requests at all right now. The caller
  gets the error, and the request is never passed on.

This example builds two small services, one for each non-trivial outcome,
and prints every step so you can watch them in the console.

## Backpressure: `ConcurrencyLimit`

[`src/limit.rs`](src/limit.rs) wraps another service and lets at most `max`
calls run through it at once. While every slot is taken, its `ready()` waits:

```rust
async fn ready(&self, ctx: Ctx<'_, Self, St>) -> Result<(), Self::Error> {
    if !self.slots.is_available() {
        self.slots.available().await;
    }
    ctx.ready(&self.service).await
}
```

Each call holds a slot for as long as it runs. The slot is a guard, so it is
also released when the call is cancelled.

There is one more detail. ntex caches readiness: once `ready()` succeeds, the
pipeline keeps sending calls without asking again. A service whose readiness
can change must tell the pipeline when it does. `ConcurrencyLimit` owns a
`ReadyNotifier`, subscribes it in `bind()`, and calls `notify()` when the
last free slot is taken:

```rust
fn bind(&self, ctx: BindCtx<'_>) {
    ctx.subscribe(&self.slots.changed);
    ctx.bind(&self.service);
}
```

Without that notification, the next caller would be sent straight into a
full service.

The demo puts the limit at the end of a chain:

```rust
let pipeline = Pipeline::new(
    (),
    service(front).and_then(ConcurrencyLimit::new(2, worker())),
);
```

`front` is always ready, but a chain is ready only when every service in it
is. So the limit at the end also holds back new work at the start. `front`
accepts a new job only after a running one finishes.

ntex ships the same idea ready-made as `ntex::util::InFlight`. This version
is written out by hand so you can read and trace it.

## Failing readiness: `Gate`

[`src/gate.rs`](src/gate.rs) wraps a service and can be opened and closed
through a `GateHandle`. While the gate is closed, `ready()` returns
`GateError::Closed`:

```rust
async fn ready(&self, ctx: Ctx<'_, Self, St>) -> Result<(), Self::Error> {
    if self.handle.0.open.get() {
        ctx.ready(&self.service).await.map_err(GateError::Service)
    } else {
        Err(GateError::Closed)
    }
}
```

`pipeline.call()` returns that error to the caller, and the wrapped service
never sees the request. Real services use this for maintenance mode, a lost
upstream connection, or an open circuit breaker. `open()` and `close()` call
`notify()`, so the pipeline checks readiness again on the next call.

## Run the example

```bash
cargo run -p service-readiness
```

Each line shows the time since start, the component that printed it, and
what happened. In the first demo, five jobs are sent at once to a limit of
two:

```text
== Backpressure: at most 2 jobs at a time ==
[    0ms] client | sending job 1
...
[    0ms] client | sending job 5
[    0ms] front  | accepted job 1
[    0ms] limit  | slot taken: 1/2 in flight
[    0ms] worker | job 1 started
[    1ms] front  | accepted job 2
[    1ms] limit  | slot taken: 2/2 in flight
[    1ms] limit  | limit reached, invalidating readiness
[    1ms] worker | job 2 started
[    1ms] limit  | not ready: 2/2 in flight, callers wait
[  115ms] worker | job 2 finished
[  115ms] limit  | slot freed: 1/2 in flight
[  115ms] limit  | ready again: a slot is free
[  115ms] front  | accepted job 3
[  115ms] limit  | slot taken: 2/2 in flight
[  115ms] limit  | limit reached, invalidating readiness
[  115ms] worker | job 3 started
[  115ms] limit  | not ready: 2/2 in flight, callers wait
[  309ms] worker | job 1 finished
...
[  433ms] client | result: Ok(1)
...
[  433ms] client | result: Ok(5)
```

Jobs 3 to 5 are not accepted by `front` until a slot frees up. They wait
with their callers, not inside the chain. Each job starts as soon as an
earlier one finishes, so job 3 takes the slot that job 2 frees.

The second demo calls a gated worker while the gate is open, closed, and
open again:

```text
== Failing readiness: a gate that can be closed ==
[  433ms] client | sending job 10
[  433ms] worker | job 10 started
[  502ms] worker | job 10 finished
[  502ms] client | result: Ok(10)
[  502ms] gate   | closing
[  502ms] client | sending job 11
[  502ms] gate   | not ready: closed, failing the readiness check
[  502ms] client | result: Err(Closed)
[  502ms] gate   | opening
[  502ms] client | sending job 12
[  502ms] worker | job 12 started
[  567ms] worker | job 12 finished
[  567ms] client | result: Ok(12)
```

Job 11 never reaches the worker. Exact timings vary with the timer
resolution of your system.

## Learn more

- [Ready before the call](https://docs.ntex.rs/1-service.html#ready-before-the-call)
- [Readiness and backpressure](https://docs.ntex.rs/2-pipeline.html#readiness-and-backpressure)
- [Pipeline readiness](https://docs.ntex.rs/2-pipeline.html#pipeline-readiness)
- [Readiness between services](https://docs.ntex.rs/2-pipeline.html#readiness-between-services)

## Tests

```sh
cargo test -p service-readiness
```

Covers the concurrency cap, readiness invalidation when the limit is
reached, and a closed gate that rejects requests without calling the wrapped
service.
