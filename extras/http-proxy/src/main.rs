//! Forward HTTP requests to a configurable upstream server.
//!
//! Look at `forward` for path, query, and header forwarding with `Client`.
//! Both request and response bodies are buffered rather than streamed.
//! Run with `cargo run -- 127.0.0.1 8080 127.0.0.1 3000` to forward
//! <http://127.0.0.1:8080> to an existing upstream on port 3000.

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

fn proxy_route() -> web::Route<AppState> {
    web::route().to_with_state(forward)
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
            .default_service(proxy_route())
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

#[cfg(test)]
mod tests {
    use super::*;
    use ntex::http::{StatusCode, header};
    use ntex::web::test;

    /// STATEMENT: The proxy preserves the request and forwards the upstream response contract.
    ///
    /// HOW:
    /// - Start a local upstream that echoes the method, URI, custom header, and request body.
    /// - POST through the real proxy app with a query and a custom request header.
    /// - Check the upstream status, content type, custom response header, and echoed body.
    /// - Check that the upstream `Connection` header is not forwarded.
    #[ntex::test]
    async fn proxy_forwards_request_and_response() {
        let upstream = test::server(async |_| {
            App::new().route(
                "/echo",
                web::post().to(async |req: HttpRequest, body: Bytes| {
                    HttpResponse::Created()
                        .content_type("text/plain")
                        .header("x-upstream", "local")
                        .header(header::CONNECTION, "close")
                        .body(format!(
                            "{} {} {} {}",
                            req.method(),
                            req.uri(),
                            req.headers().get("x-probe").unwrap().to_str().unwrap(),
                            std::str::from_utf8(&body).unwrap(),
                        ))
                }),
            )
        });
        let app = test::init_service_st(
            AppState {
                url: Url::parse(&upstream.url("/")).unwrap(),
                client: Client::new(),
            },
            App::new().default_service(proxy_route()),
        )
        .await;
        let req = test::TestRequest::post()
            .uri("/echo?name=ntex")
            .header("x-probe", "preserved")
            .payload("payload")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
        assert_eq!(
            resp.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/plain"
        );
        assert_eq!(resp.headers().get("x-upstream").unwrap(), "local");
        assert!(!resp.headers().contains_key(header::CONNECTION));
        assert_eq!(
            test::read_body(resp).await,
            "POST /echo?name=ntex preserved payload"
        );
    }

    /// STATEMENT: The proxy preserves an upstream HTTP error status and body.
    ///
    /// HOW:
    /// - Start a local upstream returning HTTP 404 with a custom body.
    /// - Request `/missing` through the real proxy app.
    /// - Check that the proxy returns the upstream's HTTP 404 and body unchanged.
    #[ntex::test]
    async fn proxy_preserves_upstream_not_found() {
        let upstream = test::server(async |_| {
            App::new().default_service(
                web::route().to(async || HttpResponse::NotFound().body("upstream missing")),
            )
        });
        let app = test::init_service_st(
            AppState {
                url: Url::parse(&upstream.url("/")).unwrap(),
                client: Client::new(),
            },
            App::new().default_service(proxy_route()),
        )
        .await;
        let req = test::TestRequest::get().uri("/missing").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        assert_eq!(test::read_body(resp).await, "upstream missing");
    }
}
