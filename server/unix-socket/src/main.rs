use ntex::web::{self, App, HttpRequest, middleware};

async fn index(_req: HttpRequest) -> &'static str {
    "Hello world!"
}

#[ntex::main]
#[cfg(unix)]
async fn main() -> std::io::Result<()> {
    unsafe {
        ::std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    web::server(async |_| {
        App::new()
            // enable logger
            .middleware(middleware::Logger::default())
            .service((
                web::resource("/index.html").route(web::get().to(|| async { "Hello world!" })),
                web::resource("/").to(index),
            ))
    })
    .bind_uds("/tmp/ntex-uds.socket", ntex::SharedCfg::new("UDS"))?
    .run()
    .await
}

#[cfg(not(unix))]
fn main() -> std::io::Result<()> {
    println!("not supported");
    Ok(())
}
