# JSON extraction and parsing

Compare ntex `types::Json<MyObj>` extraction with manually reading a
`types::Payload` stream and parsing a buffered `Bytes` body. `src/main.rs`
uses both `serde_json` and the `json` crate to return JSON responses.

## Run the example

```sh
cd extractors/json
cargo run
```

The server listens on `127.0.0.1:8080`.

## Try it

| Request | Result |
| --- | --- |
| `POST /` | Accept a JSON object with string `name` and signed 32-bit integer `number`; echo the deserialized object |
| `POST /extractor` | Same body and response as `/`, using `types::Json<MyObj>` |
| `POST /extractor2` | Same body and response; also inspect the `HttpRequest` |
| `POST /manual` | Same object, buffered manually and parsed with `serde_json` |
| `POST /mjsonrust` | Accept any UTF-8 JSON value, parsed and returned with the `json` crate |

The shared `JsonConfig` limits the three typed-extractor routes to 4,096
bytes. `/manual` checks its own 262,144-byte (256 KiB) limit and returns
HTTP 400 on overflow.

Send the typed body to any of the first four routes:

```sh
curl http://127.0.0.1:8080/ \
  -H "Content-Type: application/json" \
  -d '{"name":"Test user","number":100}'
curl http://127.0.0.1:8080/extractor \
  -H "Content-Type: application/json" \
  -d '{"name":"Test user","number":100}'
curl http://127.0.0.1:8080/extractor2 \
  -H "Content-Type: application/json" \
  -d '{"name":"Test user","number":100}'
curl http://127.0.0.1:8080/manual \
  -H "Content-Type: application/json" \
  -d '{"name":"Test user","number":100}'
```

The `json`-crate route also accepts other JSON shapes:

```sh
curl http://127.0.0.1:8080/mjsonrust \
  -H "Content-Type: application/json" \
  -d '[1,2,3]'
curl -i http://127.0.0.1:8080/mjsonrust \
  -H "Content-Type: application/json" \
  -d '{notjson}'
```

Malformed JSON or a body that is not valid UTF-8 on `/mjsonrust` produces a
JSON object with an `err` field and HTTP 200.

## Python client

The optional `client.py` posts the same typed object to `/`, prints the
response, and checks for HTTP 200. It requires Python 3 and `aiohttp`.
With the server running, use another terminal from the repository root:

```sh
cd extractors/json
python3 -m pip install aiohttp
python3 client.py
```

## Tests

```sh
cargo test -p json-example
```

Covers typed and manual JSON responses, malformed typed JSON, and the `err` responses for malformed JSON and invalid UTF-8.

## Learn more

- [Extractors](https://docs.ntex.rs/8.4-web-app.html#extractors)
- [Request bodies](https://docs.ntex.rs/8.4-web-app.html#request-bodies)
