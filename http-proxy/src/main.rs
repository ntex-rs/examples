use std::{io, net::ToSocketAddrs};

use clap::{Arg, value_t};
use ntex::client::Client;
use ntex::util::Bytes;
use ntex::web::{self, App, HttpRequest, HttpResponse, WebError, middleware, types};
use url::Url;

type Error = WebError<AppState>;

#[derive(Clone)]
struct AppState {
    url: Url,
    client: Client,
}

impl web::State for AppState {
    type Error = web::DefaultError;
}

async fn forward(
    req: HttpRequest,
    body: Bytes,
    st: types::State<AppState>,
) -> Result<HttpResponse, Error> {
    let mut new_url = st.url.clone();
    new_url.set_path(req.uri().path());
    new_url.set_query(req.uri().query());

    // TODO: This forwarded implementation is incomplete as it only handles the inofficial
    // X-Forwarded-For header but not the official Forwarded one.
    let forwarded_req = st
        .client
        .request_from(new_url.as_str(), req.head())
        .no_decompress();
    let forwarded_req = if let Some(addr) = req.head().peer_addr() {
        forwarded_req.header("x-forwarded-for", format!("{}", addr.ip()))
    } else {
        forwarded_req
    };

    let res = forwarded_req
        .send_body(body)
        .await
        .map_err(Error::from_err)?;

    let mut client_resp = HttpResponse::builder(res.status());
    // Remove `Connection` as per
    // https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Connection#Directives
    for (header_name, header_value) in res.headers().iter().filter(|(h, _)| *h != "connection") {
        client_resp.header(header_name.clone(), header_value.clone());
    }

    Ok(client_resp.body(res.body().await.map_err(Error::from_err)?))
}

#[ntex::main]
async fn main() -> io::Result<()> {
    let matches = clap::App::new("HTTP Proxy")
        .arg(
            Arg::with_name("listen_addr")
                .takes_value(true)
                .value_name("LISTEN ADDR")
                .index(1)
                .required(true),
        )
        .arg(
            Arg::with_name("listen_port")
                .takes_value(true)
                .value_name("LISTEN PORT")
                .index(2)
                .required(true),
        )
        .arg(
            Arg::with_name("forward_addr")
                .takes_value(true)
                .value_name("FWD ADDR")
                .index(3)
                .required(true),
        )
        .arg(
            Arg::with_name("forward_port")
                .takes_value(true)
                .value_name("FWD PORT")
                .index(4)
                .required(true),
        )
        .get_matches();

    let listen_addr = matches.value_of("listen_addr").unwrap();
    let listen_port = value_t!(matches, "listen_port", u16).unwrap_or_else(|e| e.exit());

    let forwarded_addr = matches.value_of("forward_addr").unwrap();
    let forwarded_port = value_t!(matches, "forward_port", u16).unwrap_or_else(|e| e.exit());

    let forward_url = Url::parse(&format!(
        "http://{}",
        (forwarded_addr, forwarded_port)
            .to_socket_addrs()
            .unwrap()
            .next()
            .unwrap()
    ))
    .unwrap();

    web::server(async move |_| {
        App::new()
            .middleware(middleware::Logger::default())
            .default_service(web::route().to(forward))
            .build_with(AppState {
                url: forward_url.clone(),
                client: Client::new(),
            })
    })
    .bind((listen_addr, listen_port), ntex::SharedCfg::new("PROXY"))?
    .run()
    .await
}
