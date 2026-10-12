use std::io::Write;

use futures::{StreamExt, TryStreamExt};
use ntex::web::{self, App, HttpResponse, WebError, middleware};
use ntex_multipart::Multipart;

async fn save_file(mut payload: Multipart) -> Result<HttpResponse, WebError> {
    // iterate over multipart stream
    while let Ok(Some(mut field)) = payload.try_next().await {
        let filename = "somename";
        let filepath = format!("./tmp/{}", filename);
        // File::create is blocking operation, use threadpool
        let mut f = web::block(|| std::fs::File::create(filepath))
            .await
            .unwrap();
        // Field in turn is stream of *Bytes* object
        while let Some(chunk) = field.next().await {
            let data = chunk.unwrap();
            // filesystem operations are blocking, we have to use threadpool
            f = web::block(move || f.write_all(&data).map(|_| f))
                .await
                .map_err(WebError::from_err)?;
        }
    }
    Ok(HttpResponse::Ok().into())
}

async fn index() -> HttpResponse {
    let html = r#"<html>
        <head><title>Upload Test</title></head>
        <body>
            <form target="/" method="post" enctype="multipart/form-data">
                <input type="file" multiple name="file"/>
                <input type="submit" value="Submit"></button>
            </form>
        </body>
    </html>"#;

    HttpResponse::Ok().body(html)
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    std::fs::create_dir_all("./tmp").unwrap();

    let ip = "0.0.0.0:3000";

    web::server(async |_| {
        App::new()
            .middleware(middleware::Logger::default())
            .service(
                web::resource("/")
                    .route(web::get().to(index))
                    .route(web::post().to(save_file)),
            )
    })
    .bind(ip, ntex::SharedCfg::new("S"))?
    .run()
    .await
}
