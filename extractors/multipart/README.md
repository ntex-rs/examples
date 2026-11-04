# Multipart file uploads

Read multipart form fields as streams with `ntex_multipart::Multipart`.
`src/main.rs` uses `web::block` for blocking file creation and writes,
while the request handler reads each field chunk asynchronously.

## Run the example

```sh
cd extractors/multipart
cargo run
```

The server listens on `0.0.0.0:3000` (all IPv4 interfaces), not just
loopback. Startup creates `./tmp` relative to the current working directory;
with the command above, that is `extractors/multipart/tmp`.

## Try it

| Request | Result |
| --- | --- |
| `GET /` | Return the upload form HTML |
| `POST /` | Save each multipart field to its own file and return the saved paths, one per line |

The form is at <http://127.0.0.1:3000/>. To upload an existing file with curl,
run this from `extractors/multipart` in another terminal:

```sh
curl -i -F "file=@README.md" http://127.0.0.1:3000/
```

The handler ignores the client-supplied filenames and saves each field as
`./tmp/<uuid>`, relative to the directory the server was started from. Errors
from the multipart stream, file creation and writes are returned to the client
instead of being ignored. The upload directory is supplied through
`web::AppState`; the executable uses `./tmp`. Requests with malformed multipart
content (for example, a missing boundary) return HTTP 400.

This is a minimal demonstration, not a production uploader: it does not limit
upload size or keep the original filenames.

## Tests

```sh
cargo test -p multipart-example
```

Covers the upload form, multi-field uploads in an automatically cleaned directory, and rejection of multipart requests without a boundary.

## Learn more

- [Request bodies](https://docs.ntex.rs/8.4-web-app.html#request-bodies)
