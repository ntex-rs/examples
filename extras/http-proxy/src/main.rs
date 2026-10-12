use std::{io, net::ToSocketAddrs};

use clap::{Arg, Command, value_parser};
use ntex::client::Client;
use ntex::util::Bytes;
use ntex::web::{self, App, HttpRequest, HttpResponse, WebError, middleware};
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
    st: &AppState,
    _: (),
    req: HttpRequest,
    body: Bytes,
) -> Result<HttpResponse, Error> {
    let mut new_url = st.url.clone();
    new_url.set_path(req.path());
    new_url.set_query(req.uri().query().map(|q| q.as_str()));

    // TODO: This forwarding implementation only handles the unofficial
    // X-Forwarded-For header, not the standard Forwarded header.
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
    let matches = Command::new("HTTP Proxy")
        .arg(
            Arg::new("listen_addr")
                .value_name("LISTEN ADDR")
                .index(1)
                .required(true),
        )
        .arg(
            Arg::new("listen_port")
                .value_name("LISTEN PORT")
                .value_parser(value_parser!(u16))
                .index(2)
                .required(true),
        )
        .arg(
            Arg::new("forward_addr")
                .value_name("FWD ADDR")
                .index(3)
                .required(true),
        )
        .arg(
            Arg::new("forward_port")
                .value_name("FWD PORT")
                .value_parser(value_parser!(u16))
                .index(4)
                .required(true),
        )
        .get_matches();

    let listen_addr = matches.get_one::<String>("listen_addr").unwrap().clone();
    let listen_port = *matches.get_one::<u16>("listen_port").unwrap();

    let forwarded_addr = matches.get_one::<String>("forward_addr").unwrap();
    let forwarded_port = *matches.get_one::<u16>("forward_port").unwrap();

    let forward_url = Url::parse(&format!(
        "http://{}",
        (forwarded_addr.as_str(), forwarded_port)
            .to_socket_addrs()
            .unwrap()
            .next()
            .unwrap()
    ))
    .unwrap();

    web::server(async move |_| {
        App::new()
            .middleware(middleware::Logger::default())
            .default_service(web::route().to_with_state(forward))
            .build_with(AppState {
                url: forward_url.clone(),
                client: Client::new(),
            })
    })
    .bind(
        (listen_addr.as_str(), listen_port),
        ntex::SharedCfg::new("PROXY"),
    )?
    .run()
    .await
}
