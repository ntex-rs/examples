//! Dispatch JSON-RPC-style requests over HTTP with a shared counter.
//!
//! Look at `rpc_handler` for response construction, `rpc_select` for
//! method dispatch, and `RpcState` for state shared between workers.
//! Run with `cargo run` and POST JSON to <http://127.0.0.1:8080>.

#![allow(clippy::type_complexity)]

use std::sync::Arc;
use std::{error, pin::Pin, sync::RwLock, time::Duration};

use futures::{Future, FutureExt};
use ntex::web::{self, App, HttpResponse, WebError, middleware};
use ntex::{time::sleep, util::Bytes};
use serde_json::Value;

#[allow(dead_code)]
mod convention;

/// The main handler for JSONRPC server.
async fn rpc_handler(app_state: &AppState, _: (), body: Bytes) -> Result<HttpResponse, Error> {
    let reqjson: convention::Request = match serde_json::from_slice(body.as_ref()) {
        Ok(ok) => ok,
        Err(_) => {
            let r = convention::Response {
                jsonrpc: String::from(convention::JSONRPC_VERSION),
                result: Value::Null,
                error: Some(convention::ErrorData::std(-32700)),
                id: Value::Null,
            };
            return Ok(HttpResponse::Ok()
                .content_type("application/json")
                .body(r.dump()));
        }
    };
    let mut result = convention::Response {
        id: reqjson.id.clone(),
        ..Default::default()
    };

    match rpc_select(app_state, reqjson.method.as_str(), reqjson.params).await {
        Ok(ok) => result.result = ok,
        Err(e) => result.error = Some(e),
    }

    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .body(result.dump()))
}

async fn rpc_select(
    app_state: &RpcState,
    method: &str,
    params: Vec<Value>,
) -> Result<Value, convention::ErrorData> {
    match method {
        "ping" => {
            let r = app_state.network.read().unwrap().ping();
            Ok(Value::from(r))
        }
        "wait" => {
            if params.len() != 1 || !params[0].is_u64() {
                return Err(convention::ErrorData::std(-32602));
            }

            // Take the future out of the read guard so the lock is released
            // before awaiting.
            let wait = app_state
                .network
                .read()
                .unwrap()
                .wait(params[0].as_u64().unwrap());
            match wait.await {
                Ok(ok) => Ok(Value::from(ok)),
                Err(e) => Err(convention::ErrorData::new(500, &format!("{:?}", e)[..])),
            }
        }
        "get" => {
            let r = app_state.network.read().unwrap().get();
            Ok(Value::from(r))
        }
        "inc" => {
            app_state.network.write().unwrap().inc();
            Ok(Value::Null)
        }
        _ => Err(convention::ErrorData::std(-32601)),
    }
}

pub struct ObjNetwork {
    c: u32,
}

impl ObjNetwork {
    fn new() -> Self {
        Self { c: 0 }
    }
}

impl ObjNetwork {
    fn ping(&self) -> String {
        String::from("pong")
    }

    fn wait(&self, d: u64) -> Pin<Box<dyn Future<Output = Result<String, Box<dyn error::Error>>>>> {
        async move {
            sleep(Duration::from_secs(d)).await;
            Ok(String::from("pong"))
        }
        .boxed_local()
    }

    fn get(&self) -> u32 {
        self.c
    }

    fn inc(&mut self) {
        self.c += 1;
    }
}

pub struct RpcState {
    network: RwLock<ObjNetwork>,
}

impl RpcState {
    pub fn new(network: RwLock<ObjNetwork>) -> Self {
        Self { network }
    }
}

type AppState = web::AppState<Arc<RpcState>>;
type Error = WebError<AppState>;

fn app_config(config: &mut web::ServiceConfig<AppState>) {
    config.service(web::resource("/").route(web::post().to_with_state(rpc_handler)));
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let app_state = Arc::new(RpcState::new(RwLock::new(ObjNetwork::new())));

    web::server(async move |_| {
        App::new()
            .middleware(middleware::Logger::default())
            .configure(app_config)
            .build_with(AppState::new(app_state.clone()))
    })
    .bind("127.0.0.1:8080", ntex::SharedCfg::new("JSONRPC"))
    .unwrap()
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use ntex::http::{StatusCode, header};
    use ntex::web::test;
    use serde_json::json;

    fn state() -> AppState {
        AppState::new(Arc::new(RpcState::new(RwLock::new(ObjNetwork::new()))))
    }

    /// STATEMENT: Successful RPC methods return their results and share the app's counter.
    ///
    /// HOW:
    /// - Initialize the real app with a fresh counter and call `ping`, `get`, `inc`, and `get`.
    /// - Check HTTP 200, JSON content type, request IDs, and a null error on each response.
    /// - Check that `ping` returns pong and `inc` changes the counter from zero to one.
    #[ntex::test]
    async fn successful_methods_share_counter_state() {
        let app = test::init_service_st(state(), App::new().configure(app_config)).await;
        for (id, method, result) in [
            (1, "ping", json!("pong")),
            (2, "get", json!(0)),
            (3, "inc", Value::Null),
            (4, "get", json!(1)),
        ] {
            let req = test::TestRequest::post()
                .uri("/")
                .json(&json!({"jsonrpc": "2.0", "method": method, "params": [], "id": id}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), StatusCode::OK);
            assert_eq!(
                resp.headers().get(header::CONTENT_TYPE).unwrap(),
                "application/json"
            );
            let body: Value = serde_json::from_slice(&test::read_body(resp).await).unwrap();
            assert_eq!(
                body,
                json!({"jsonrpc": "2.0", "result": result, "error": null, "id": id})
            );
        }
    }

    /// STATEMENT: Unknown RPC methods return a method-not-found error with the original ID.
    ///
    /// HOW:
    /// - POST an unknown method to the real app.
    /// - Check HTTP 200, JSON content type, the original ID, and error code `-32601`.
    #[ntex::test]
    async fn unknown_method_returns_rpc_error() {
        let app = test::init_service_st(state(), App::new().configure(app_config)).await;
        let req = test::TestRequest::post()
            .uri("/")
            .json(&json!({"jsonrpc": "2.0", "method": "missing", "params": [], "id": "request"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
        let body: Value = serde_json::from_slice(&test::read_body(resp).await).unwrap();
        assert_eq!(body["id"], "request");
        assert!(body["result"].is_null());
        assert_eq!(body["error"]["code"], -32601);
        assert_eq!(body["error"]["message"], "Method not found");
    }

    /// STATEMENT: Malformed RPC requests return a parse error rather than an HTTP failure.
    ///
    /// HOW:
    /// - POST invalid JSON and an object missing required request fields to the real app.
    /// - Check HTTP 200, JSON content type, null IDs, and parse error code `-32700`.
    #[ntex::test]
    async fn malformed_request_returns_parse_error() {
        let app = test::init_service_st(state(), App::new().configure(app_config)).await;
        for payload in ["{", "{}"] {
            let req = test::TestRequest::post()
                .uri("/")
                .header(header::CONTENT_TYPE, "application/json")
                .payload(payload)
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), StatusCode::OK);
            assert_eq!(
                resp.headers().get(header::CONTENT_TYPE).unwrap(),
                "application/json"
            );
            let body: Value = serde_json::from_slice(&test::read_body(resp).await).unwrap();
            assert!(body["id"].is_null());
            assert!(body["result"].is_null());
            assert_eq!(body["error"]["code"], -32700);
            assert_eq!(body["error"]["message"], "Parse error");
        }
    }

    /// STATEMENT: The `wait` RPC rejects invalid parameters without waiting.
    ///
    /// HOW:
    /// - Call `wait` through the real app with no parameter and with a string parameter.
    /// - Check HTTP 200 and error code `-32602` in both responses.
    #[ntex::test]
    async fn wait_rejects_invalid_parameters() {
        let app = test::init_service_st(state(), App::new().configure(app_config)).await;
        for params in [json!([]), json!(["not a duration"])] {
            let req = test::TestRequest::post()
                .uri("/")
                .json(&json!({"jsonrpc": "2.0", "method": "wait", "params": params, "id": 1}))
                .to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), StatusCode::OK);
            let body: Value = serde_json::from_slice(&test::read_body(resp).await).unwrap();
            assert_eq!(body["error"]["code"], -32602);
        }
    }
}
