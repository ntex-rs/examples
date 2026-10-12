use std::sync::{Arc, atomic::AtomicUsize, atomic::Ordering};
use std::{io, net::SocketAddr, rc::Rc, time::Instant};

use ntex::io::{Io, Layer, types::PeerAddr};
use ntex::service::{Pipeline, Service, State};
use ntex::tls::openssl::{SslAcceptor, SslFilter};
use ntex::web::{self, App, HttpRequest, HttpResponse, middleware};
use ntex::{SharedCfg, http, server::ServerAppConfig};
use openssl::ssl::{self, SslFiletype, SslMethod};
use uuid::Uuid;

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
        // load ssl keys
        let mut builder = ssl::SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
        builder
            .set_private_key_file("key.pem", SslFiletype::PEM)
            .unwrap();
        builder.set_certificate_chain_file("cert.pem").unwrap();

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
                                .service(web::resource("/").to_with_state(index)),
                        )
                        .map_err(|e| io::Error::other(format!("{e}"))),
                    )
            },
        )?
        .run()
        .await
}
