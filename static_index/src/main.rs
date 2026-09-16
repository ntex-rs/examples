use ntex::web::{self, App, middleware};
use ntex_files as fs;

#[ntex::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    web::server(async |_| {
        App::new()
            // enable logger
            .middleware(middleware::Logger::default())
            .service(
                // static files
                fs::Files::new("/", "./static/").index_file("index.html"),
            )
    })
    .bind("127.0.0.1:8080", ntex::SharedCfg::new("S"))?
    .run()
    .await
}
