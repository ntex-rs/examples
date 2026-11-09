//! A service that limits how many requests run at the same time.
//!
//! [`ConcurrencyLimit`] wraps another service and lets at most `max` calls run
//! through it at once. When every slot is taken, the service reports that it
//! is *not ready*. Callers wait for readiness before they hand over a request,
//! so extra requests wait with their callers instead of piling up inside the
//! service.
//!
//! ntex caches readiness: after `ready()` succeeds, the pipeline keeps
//! dispatching calls without asking again. A service whose readiness can change
//! must therefore tell the pipeline when it does. This one owns a
//! [`ReadyNotifier`], subscribes it in [`Service::bind`], and notifies it when
//! the last free slot is taken. The next caller then runs `ready()` again and
//! waits.
//!
//! The same idea is available ready-made as `ntex::util::InFlight`; this
//! version is written out by hand, with console output, so you can watch it
//! work.

use std::cell::{Cell, RefCell};
use std::future::poll_fn;
use std::rc::Rc;
use std::task::{Poll, Waker};

use ntex::service::{BindCtx, Ctx, ReadyBinding, ReadyNotifier, Service};

/// Limits the number of calls that run through the wrapped service at once.
///
/// The service is ready while fewer than `max` calls are in flight and the
/// wrapped service is ready. Errors and responses come from the wrapped
/// service unchanged.
///
/// # Example
///
/// ```rust,ignore
/// let pipeline = Pipeline::new((), ConcurrencyLimit::new(2, worker()));
///
/// // Five calls at once: two run, three wait for readiness.
/// let results = join_all((0..5).map(|id| pipeline.call(Job::new(id, 100)))).await;
/// ```
pub struct ConcurrencyLimit<S> {
    slots: Rc<Slots>,
    service: ReadyBinding<S>,
}

impl<S> ConcurrencyLimit<S> {
    /// Wraps `service` and allows at most `max` calls through it at once.
    ///
    /// # Panics
    ///
    /// Panics if `max` is zero, because such a service would never be ready.
    pub fn new(max: usize, service: S) -> Self {
        assert!(max > 0, "the limit must allow at least one call");

        Self {
            slots: Rc::new(Slots {
                max,
                in_flight: Cell::new(0),
                waiters: RefCell::new(Vec::new()),
                changed: ReadyNotifier::new(),
            }),
            // `ReadyBinding` gives the wrapped service its own readiness scope,
            // so the pipeline can track it separately from this wrapper.
            service: ReadyBinding::new(service),
        }
    }
}

impl<S, St, Req> Service<St, Req> for ConcurrencyLimit<S>
where
    S: Service<St, Req>,
{
    type Res = S::Res;
    type Error = S::Error;

    /// Registers this service with the pipeline.
    ///
    /// Subscribing to the notifier lets [`Slots::acquire`] invalidate the
    /// pipeline's cached readiness. Binding the inner service links its
    /// readiness to ours.
    fn bind(&self, ctx: BindCtx<'_>) {
        ctx.subscribe(&self.slots.changed);
        ctx.bind(&self.service);
    }

    /// Ready when a slot is free and the wrapped service is ready.
    ///
    /// While every slot is taken this future stays pending. Releasing a slot
    /// wakes it, and it then checks the wrapped service as well.
    async fn ready(&self, ctx: Ctx<'_, Self, St>) -> Result<(), Self::Error> {
        if !self.slots.is_available() {
            log!(
                "limit",
                "not ready: {}/{} in flight, callers wait",
                self.slots.in_flight.get(),
                self.slots.max
            );
            self.slots.available().await;
            log!("limit", "ready again: a slot is free");
        }
        ctx.ready(&self.service).await
    }

    /// Takes a slot for the duration of the call.
    ///
    /// The slot is released when the call finishes or is cancelled, because
    /// [`SlotGuard`] gives it back on drop.
    async fn call(&self, req: Req, ctx: Ctx<'_, Self, St>) -> Result<Self::Res, Self::Error> {
        // A caller that skipped readiness could arrive while every slot is
        // taken. Wait for a slot here, so the limit holds in that case too.
        if !self.slots.is_available() {
            self.slots.available().await;
        }

        let _slot = self.slots.acquire();
        ctx.call(&self.service, req).await
    }

    /// Forwards shutdown to the wrapped service.
    async fn shutdown(&self, ctx: Ctx<'_, Self, St>) {
        ctx.shutdown(&self.service).await;
    }
}

/// Slot accounting shared by a [`ConcurrencyLimit`] and its [`SlotGuard`]s.
struct Slots {
    max: usize,
    in_flight: Cell<usize>,
    /// Tasks waiting for a free slot. They are all woken when one frees up and
    /// check again.
    waiters: RefCell<Vec<Waker>>,
    /// Invalidates the pipeline's cached readiness.
    changed: ReadyNotifier,
}

impl Slots {
    fn is_available(&self) -> bool {
        self.in_flight.get() < self.max
    }

    /// Waits until at least one slot is free.
    async fn available(&self) {
        poll_fn(|cx| {
            if self.is_available() {
                Poll::Ready(())
            } else {
                self.waiters.borrow_mut().push(cx.waker().clone());
                Poll::Pending
            }
        })
        .await;
    }

    /// Takes a slot and returns a guard that gives it back on drop.
    fn acquire(self: &Rc<Self>) -> SlotGuard {
        let in_flight = self.in_flight.get() + 1;
        self.in_flight.set(in_flight);
        log!("limit", "slot taken: {in_flight}/{} in flight", self.max);

        if in_flight == self.max {
            // The pipeline still holds a cached "ready" from the last check.
            // Without this notification, the next caller would be dispatched
            // straight into a full service.
            log!("limit", "limit reached, invalidating readiness");
            self.changed.notify();
        }

        SlotGuard(self.clone())
    }
}

/// Holds one slot of a [`ConcurrencyLimit`] and releases it on drop.
struct SlotGuard(Rc<Slots>);

impl Drop for SlotGuard {
    fn drop(&mut self) {
        let slots = &self.0;
        let in_flight = slots.in_flight.get() - 1;
        slots.in_flight.set(in_flight);
        log!("limit", "slot freed: {in_flight}/{} in flight", slots.max);

        // Readiness was already invalidated when the limit was reached, and the
        // pending readiness check is registered as a waiter. Waking it is
        // enough; there is no cached "ready" to clear.
        for waker in slots.waiters.borrow_mut().drain(..) {
            waker.wake();
        }
    }
}
