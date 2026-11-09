//! A service whose readiness can fail.
//!
//! [`Gate`] wraps another service and can be opened or closed from outside
//! through a [`GateHandle`]. While the gate is closed, `ready()` returns
//! [`GateError::Closed`]. The pipeline passes that error to the caller and the
//! request never reaches the wrapped service.
//!
//! A failed readiness check means "this service can't accept requests at all
//! right now". It is different from a pending check, which means "not yet,
//! wait". Use it for states such as maintenance mode, a lost upstream
//! connection, or an open circuit breaker.
//!
//! Like [`ConcurrencyLimit`](crate::limit::ConcurrencyLimit), the gate must
//! notify the pipeline when its state changes. Otherwise the pipeline would
//! keep using its cached readiness, and a freshly closed gate would still let
//! requests through.

use std::cell::Cell;
use std::{fmt, rc::Rc};

use ntex::service::{BindCtx, Ctx, ReadyBinding, ReadyNotifier, Service};

/// The error returned by a [`Gate`].
#[derive(Debug, PartialEq, Eq)]
pub enum GateError<E> {
    /// The gate is closed. Returned from the readiness check, so the request
    /// was never passed on.
    Closed,
    /// The wrapped service failed.
    Service(E),
}

impl<E: fmt::Display> fmt::Display for GateError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GateError::Closed => f.write_str("the gate is closed"),
            GateError::Service(e) => write!(f, "service error: {e}"),
        }
    }
}

/// Opens and closes a [`Gate`].
///
/// Clones control the same gate. Create the handle first, keep a clone, and
/// pass another clone to [`Gate::new`].
#[derive(Clone)]
pub struct GateHandle(Rc<GateState>);

struct GateState {
    open: Cell<bool>,
    changed: ReadyNotifier,
}

impl GateHandle {
    /// Creates a handle for an open gate.
    pub fn new() -> Self {
        Self(Rc::new(GateState {
            open: Cell::new(true),
            changed: ReadyNotifier::new(),
        }))
    }

    /// Opens the gate. The next readiness check succeeds again.
    pub fn open(&self) {
        log!("gate", "opening");
        self.set(true);
    }

    /// Closes the gate. The next readiness check fails with
    /// [`GateError::Closed`].
    pub fn close(&self) {
        log!("gate", "closing");
        self.set(false);
    }

    fn set(&self, open: bool) {
        if self.0.open.replace(open) != open {
            // Make the pipeline run `ready()` again instead of trusting the
            // result of the last check.
            self.0.changed.notify();
        }
    }
}

impl Default for GateHandle {
    fn default() -> Self {
        Self::new()
    }
}

/// Fails readiness while its [`GateHandle`] is closed.
///
/// # Example
///
/// ```rust,ignore
/// let gate = GateHandle::new();
/// let pipeline = Pipeline::new((), Gate::new(gate.clone(), worker()));
///
/// gate.close();
/// assert_eq!(pipeline.call(job).await, Err(GateError::Closed));
/// ```
pub struct Gate<S> {
    handle: GateHandle,
    service: ReadyBinding<S>,
}

impl<S> Gate<S> {
    /// Wraps `service`. The gate starts in whatever state `handle` is in.
    pub fn new(handle: GateHandle, service: S) -> Self {
        Self {
            handle,
            service: ReadyBinding::new(service),
        }
    }
}

impl<S, St, Req> Service<St, Req> for Gate<S>
where
    S: Service<St, Req>,
{
    type Res = S::Res;
    type Error = GateError<S::Error>;

    /// Subscribes to the handle's changes and binds the wrapped service.
    fn bind(&self, ctx: BindCtx<'_>) {
        ctx.subscribe(&self.handle.0.changed);
        ctx.bind(&self.service);
    }

    /// Fails while the gate is closed. Otherwise ready when the wrapped
    /// service is.
    async fn ready(&self, ctx: Ctx<'_, Self, St>) -> Result<(), Self::Error> {
        if self.handle.0.open.get() {
            ctx.ready(&self.service).await.map_err(GateError::Service)
        } else {
            log!("gate", "not ready: closed, failing the readiness check");
            Err(GateError::Closed)
        }
    }

    /// Passes the request to the wrapped service.
    async fn call(&self, req: Req, ctx: Ctx<'_, Self, St>) -> Result<Self::Res, Self::Error> {
        ctx.call(&self.service, req)
            .await
            .map_err(GateError::Service)
    }

    /// Forwards shutdown to the wrapped service.
    async fn shutdown(&self, ctx: Ctx<'_, Self, St>) {
        ctx.shutdown(&self.service).await;
    }
}
