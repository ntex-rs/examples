use ntex::web::{self, types, App, HttpResponse};
use ntex::{client::Client, connect::openssl::SslConnector, SharedCfg};
use openssl::ssl;

async fn index(client: types::State<web::AppState<Client>>) -> HttpResponse {
    let now = std::time::Instant::now();
    let payload = client
        .st()
        .get("https://upload.wikimedia.org/wikipedia/commons/f/ff/Pizigani_1367_Chart_10MB.jpg")
        .send()
        .await
        .unwrap()
        .body()
        .limit(20_000_000) // sets max allowable payload size
        .await
        .unwrap();

    println!(
        "awc time elapsed while reading bytes into memory: {} ms",
        now.elapsed().as_millis()
    );

    HttpResponse::Ok().content_type("image/jpeg").body(payload)
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    let port = 3000;

    web::server(async |_| {
        let connector = SslConnector::new(
            ssl::SslConnector::builder(ssl::SslMethod::tls())
                .unwrap()
                .build(),
        );

        let client = Client::builder()
            .secure_connector(connector)
            .build(SharedCfg::default());

        App::new()
            .service(web::resource("/").to(index))
            .build_with(web::AppState::new(client))
    })
    .bind(("0.0.0.0", port), SharedCfg::new("AWC"))?
    .run()
    .await
}
