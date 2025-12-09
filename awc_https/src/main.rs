use ntex::client::{Client, Connector};
use ntex::web::{self, App, HttpResponse};
use ntex::SharedCfg;
use openssl::ssl::{SslConnector, SslMethod};

async fn index(client: web::types::State<Client>) -> HttpResponse {
    let now = std::time::Instant::now();
    let payload =
        client
        .get("https://upload.wikimedia.org/wikipedia/commons/f/ff/Pizigani_1367_Chart_10MB.jpg")
        .send()
        .await
        .unwrap()
        .body()
        .limit(20_000_000)  // sets max allowable payload size
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

    web::server(async || {
        let builder = SslConnector::builder(SslMethod::tls()).unwrap();

        let client = Client::builder()
            .connector::<&str>(Connector::default().openssl(builder.build()))
            .build(SharedCfg::default())
            .await
            .unwrap();

        App::new()
            .state(client)
            .service(web::resource("/").to(index))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
