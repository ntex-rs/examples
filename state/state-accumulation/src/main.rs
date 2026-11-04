//! Accumulating per-connection state through a service chain.
//!
//! `connect` records a connection id, start time and peer address, `tls`
//! performs the OpenSSL handshake and extends that state, and the web handler
//! reads the result as its application state (`ConnectionWithTls`). Worker
//! resources (`AppState`) are created by `ServerAppConfig::create()`.
//!
//! Run with `cargo run -p state5`, then `curl -k https://127.0.0.1:8080/`.

use std::sync::{Arc, atomic::AtomicUsize, atomic::Ordering};
use std::{io, net::SocketAddr, rc::Rc, time::Instant};

use ntex::io::{Io, Layer, types::PeerAddr};
use ntex::service::{Pipeline, Service, State};
use ntex::tls::openssl::{SslAcceptor, SslFilter};
use ntex::web::{self, App, HttpRequest, HttpResponse, middleware};
use ntex::{SharedCfg, http, server::ServerAppConfig};
use openssl::ssl::{self, SslFiletype, SslMethod};
use uuid::Uuid;

// The self-signed test certificate shared with `tls/openssl`, resolved at
// compile time so the example can be started from any directory.
const CERT_FILE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tls/openssl/cert.pem");
const KEY_FILE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tls/openssl/key.pem");

// ========== AppState ===========

#[derive(Clone)]
struct AppState {
    ssl: Rc<Pipeline<Io, Io<Layer<SslFilter>>, io::Error>>,
    counter: Arc<AtomicUsize>,
}

impl web::State for AppState {
    type Error = web::DefaultError;
}

struct AppBuilder {
    ssl: ssl::SslAcceptor,
    counter: Arc<AtomicUsize>,
}

impl AppBuilder {
    fn new() -> Self {
        // Load the test certificate and key.
        let mut builder = ssl::SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
        builder
            .set_private_key_file(KEY_FILE, SslFiletype::PEM)
            .unwrap();
        builder.set_certificate_chain_file(CERT_FILE).unwrap();

        Self {
            ssl: builder.build(),
            counter: Arc::new(AtomicUsize::new(0usize)),
        }
    }
}

impl ServerAppConfig for AppBuilder {
    type State = AppState;

    // Create the state for one server worker.
    async fn create(&self) -> io::Result<Self::State> {
        Ok(AppState {
            ssl: Rc::new(Pipeline::new((), SslAcceptor::new(self.ssl.clone()))),
            counter: self.counter.clone(),
        })
    }
}

// ============ Connection state ===============

struct Connection {
    id: Uuid,
    created: Instant,
    peer_addr: SocketAddr,
}

async fn connect(_: &AppState, req: Io) -> io::Result<State<Connection, Io>> {
    // Load connection info
    let peer_addr = req
        .query::<PeerAddr>()
        .get()
        .ok_or_else(|| io::Error::other("Cannot get peer addr"))?
        .into_inner();

    Ok(State {
        req,
        state: Connection {
            peer_addr,
            id: Uuid::new_v4(),
            created: Instant::now(),
        },
    })
}

#[derive(Clone)]
struct ConnectionWithTls {
    id: Uuid,
    created: Instant,
    peer_addr: SocketAddr,
    counter: Arc<AtomicUsize>,
}

impl web::State for ConnectionWithTls {
    type Error = web::DefaultError;
}

async fn tls(
    st: &AppState,
    msg: State<Connection, Io>,
) -> io::Result<State<ConnectionWithTls, Io<Layer<SslFilter>>>> {
    let State { req, state } = msg;

    // Perform the TLS handshake.
    let req = st.ssl.call(req).await?;

    Ok(State {
        req,
        state: ConnectionWithTls {
            id: state.id,
            created: state.created,
            peer_addr: state.peer_addr,
            counter: st.counter.clone(),
        },
    })
}

/// Uses the connection state made available to HTTP handlers.
async fn index(st: &ConnectionWithTls, _: (), req: HttpRequest) -> HttpResponse {
    println!(
        "id: {:?} created: {:?}, peer-addr: {:?}, {req:?}",
        st.id, st.created, st.peer_addr
    );

    // Increment the counters
    st.counter.fetch_add(1, Ordering::SeqCst);

    HttpResponse::Ok().build()
}

fn config(cfg: &mut web::ServiceConfig<ConnectionWithTls>) {
    cfg.service(web::resource("/").to_with_state(index));
}

#[ntex::main]
async fn main() -> io::Result<()> {
    env_logger::init();

    // Create the process-wide configuration before building the server.
    let cfg = AppBuilder::new();

    // Start the server with the application configuration. The factory receives
    // a reference to the worker-local state. As the connection moves through the
    // service chain, connection-specific state is accumulated in ConnectionWithTls.
    // That accumulated state is then made available to HttpService and web::App.
    ntex::server::build_with_config(cfg)
        .bind(
            "HTTP",
            "127.0.0.1:8080",
            SharedCfg::new("STATE"),
            async |_: &AppState| {
                // Collect the initial connection information.
                ntex::service(connect)
                    // Perform the TLS handshake.
                    .and_then(tls)
                    // Pass the established connection to the HTTP service.
                    .and_then(
                        http::HttpService::new(
                            App::new()
                                // Enable request logging.
                                .middleware(middleware::Logger::default())
                                // Register the request handler.
                                .configure(config),
                        )
                        .map_err(|e| io::Error::other(format!("{e}"))),
                    )
            },
        )?
        .run()
        .await
}

#[cfg(test)]
mod tests {
    use ntex::http::StatusCode;
    use ntex::web::test::{self, TestRequest};

    use super::*;

    fn connection_state() -> ConnectionWithTls {
        ConnectionWithTls {
            id: Uuid::new_v4(),
            created: Instant::now(),
            peer_addr: "127.0.0.1:12345".parse().unwrap(),
            counter: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// STATEMENT: The index handler updates the counter in accumulated connection state.
    ///
    /// HOW:
    /// - Initialize the real routes with `ConnectionWithTls`, without a TLS listener.
    /// - Request `/` twice and check successful empty responses and the shared counter.
    #[ntex::test]
    async fn index_updates_connection_counter() {
        let state = connection_state();
        let app = test::init_service_st(state.clone(), App::new().configure(config)).await;

        for expected in 1..=2 {
            let response = test::call_service(&app, TestRequest::get().uri("/").to_request()).await;
            assert_eq!(response.status(), StatusCode::OK);
            assert!(test::read_body(response).await.is_empty());
            assert_eq!(state.counter.load(Ordering::SeqCst), expected);
        }
    }

    /// STATEMENT: Unknown routes leave the connection counter unchanged.
    ///
    /// HOW:
    /// - Initialize the app with explicit connection state and request `/missing`.
    /// - Check the empty `404 Not Found` response and the untouched counter.
    #[ntex::test]
    async fn unknown_route_leaves_connection_counter_unchanged() {
        let state = connection_state();
        let app = test::init_service_st(state.clone(), App::new().configure(config)).await;
        let response =
            test::call_service(&app, TestRequest::get().uri("/missing").to_request()).await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert!(test::read_body(response).await.is_empty());
        assert_eq!(state.counter.load(Ordering::SeqCst), 0);
    }
}
