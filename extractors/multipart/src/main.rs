//! Stream multipart uploads to a local file with ntex.
//!
//! `Multipart` supplies asynchronous field chunks, while `web::block` handles
//! file creation and writes. Each field is saved to `./tmp/<uuid>` relative to
//! the working directory, and the response lists the saved paths.
//! Run with `cargo run`; the server binds to all IPv4 interfaces on port 3000.

use std::{io::Write, path::PathBuf};

use futures::{StreamExt, TryStreamExt};
use ntex::web::{self, App, HttpResponse, WebError, middleware};
use ntex_multipart::Multipart;

type AppState = web::AppState<PathBuf>;
type Error = WebError<AppState>;

/// Saves every multipart field to its own file and returns the saved paths.
async fn save_file(
    directory: &AppState,
    _: (),
    mut payload: Multipart,
) -> Result<HttpResponse, Error> {
    let mut saved = Vec::new();

    // Iterate over the multipart stream; a malformed stream is reported to the client.
    while let Some(mut field) = payload.try_next().await.map_err(Error::from_err)? {
        // Ignore the client-supplied filename and use a unique name instead.
        let filepath = directory.join(uuid::Uuid::new_v4().to_string());

        // File::create is a blocking operation, so run it on the thread pool.
        let path = filepath.clone();
        let mut f = web::block(move || std::fs::File::create(path))
            .await
            .map_err(Error::from_err)?;

        // The field is a stream of `Bytes` chunks.
        while let Some(chunk) = field.next().await {
            let data = chunk.map_err(Error::from_err)?;
            // Filesystem writes are blocking too.
            f = web::block(move || f.write_all(&data).map(|_| f))
                .await
                .map_err(Error::from_err)?;
        }
        saved.push(filepath.to_string_lossy().into_owned());
    }
    Ok(HttpResponse::Ok().body(saved.join("\n")))
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

fn app_config(config: &mut web::ServiceConfig<AppState>) {
    config.service(
        web::resource("/")
            .route(web::get().to(index))
            .route(web::post().to_with_state(save_file)),
    );
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    std::fs::create_dir_all("./tmp")?;

    let ip = "0.0.0.0:3000";

    web::server(async |_| {
        App::new()
            .middleware(middleware::Logger::default())
            .configure(app_config)
            .build_with(AppState::new(PathBuf::from("./tmp")))
    })
    .bind(ip, ntex::SharedCfg::new("S"))?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use ntex::http::{StatusCode, header};
    use ntex::web::test;

    struct UploadDirectory(PathBuf);

    impl UploadDirectory {
        fn new() -> Self {
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join(format!(".test-uploads-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for UploadDirectory {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }

    /// STATEMENT: The index route serves the multipart upload form.
    ///
    /// HOW:
    /// - Initialize the real app and request `/` without creating an upload directory.
    /// - Check HTTP 200 and the multipart form encoding in the HTML body.
    #[ntex::test]
    async fn index_serves_upload_form() {
        let app = test::init_service_st(
            AppState::new(PathBuf::new()),
            App::new().configure(app_config),
        )
        .await;
        let req = test::TestRequest::get().uri("/").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = test::read_body(resp).await;
        assert!(
            std::str::from_utf8(&body)
                .unwrap()
                .contains("multipart/form-data")
        );
    }

    /// STATEMENT: Multipart uploads save each field under a unique name and return its path.
    ///
    /// HOW:
    /// - Configure the real app with a unique, automatically cleaned upload directory.
    /// - POST two fields, including a traversal-style filename, and check HTTP 200.
    /// - Check that two returned paths stay in that directory and contain the respective field bytes.
    #[ntex::test]
    async fn upload_saves_fields_and_returns_paths() {
        let directory = UploadDirectory::new();
        let app = test::init_service_st(
            AppState::new(directory.0.clone()),
            App::new().configure(app_config),
        )
        .await;
        let payload = concat!(
            "--boundary\r\n",
            "Content-Disposition: form-data; name=\"first\"; filename=\"../escape.txt\"\r\n",
            "Content-Type: text/plain\r\n\r\n",
            "first upload\r\n",
            "--boundary\r\n",
            "Content-Disposition: form-data; name=\"second\"; filename=\"second.txt\"\r\n",
            "Content-Type: text/plain\r\n\r\n",
            "second upload\r\n",
            "--boundary--\r\n",
        );
        let req = test::TestRequest::post()
            .uri("/")
            .header(
                header::CONTENT_TYPE,
                "multipart/form-data; boundary=boundary",
            )
            .payload(payload)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = test::read_body(resp).await;
        let paths: Vec<_> = std::str::from_utf8(&body)
            .unwrap()
            .lines()
            .map(PathBuf::from)
            .collect();
        assert_eq!(paths.len(), 2);
        assert_ne!(paths[0], paths[1]);
        for (path, expected) in paths.iter().zip(["first upload", "second upload"]) {
            assert_eq!(path.parent(), Some(directory.0.as_path()));
            assert_eq!(std::fs::read_to_string(path).unwrap(), expected);
        }
        assert_eq!(std::fs::read_dir(&directory.0).unwrap().count(), 2);
    }

    /// STATEMENT: Multipart extraction rejects requests without a boundary.
    ///
    /// HOW:
    /// - POST a multipart content type without its required boundary to the real app.
    /// - Check HTTP 400; no upload directory is created or used.
    #[ntex::test]
    async fn missing_boundary_is_bad_request() {
        let app = test::init_service_st(
            AppState::new(PathBuf::new()),
            App::new().configure(app_config),
        )
        .await;
        let req = test::TestRequest::post()
            .uri("/")
            .header(header::CONTENT_TYPE, "multipart/form-data")
            .payload("invalid multipart")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }
}
