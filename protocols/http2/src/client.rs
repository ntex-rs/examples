//! Calls every route of the server.
use std::error::Error;

use http2::{ADDR, BYTES_TRAILER, connect, read_response};
use ntex::http::{HeaderMap, Method};
use ntex::util::{Bytes, join};

#[ntex::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let _ = env_logger::try_init();
    let client = connect(ADDR);

    // a request without a body
    let (_, recv) = client
        .send(Method::GET, "/hello".into(), HeaderMap::new(), true)
        .await?;
    let res = read_response(recv).await?;
    println!("GET /hello   {} {:?}", res.status, res.body);

    // a body in several chunks, the last one ends the stream
    let (send, recv) = client
        .send(Method::POST, "/echo".into(), HeaderMap::new(), false)
        .await?;
    send.send_payload(Bytes::from_static(b"hello "), false)
        .await?;
    send.send_payload(Bytes::from_static(b"again"), true)
        .await?;
    let res = read_response(recv).await?;
    println!("POST /echo   {} {:?}", res.status, res.body);

    // 1 MiB, sent only as fast as the server releases flow control
    // capacity; the response is read at the same time
    let (send, recv) = client
        .send(Method::POST, "/count".into(), HeaderMap::new(), false)
        .await?;
    let total = 1024 * 1024;
    let (sent, res) = join(
        async {
            let mut left = total;
            while left > 0 {
                // wait until the server can take more
                let size = (send.send_capacity().await? as usize).min(left);
                send.send_payload(Bytes::from(vec![b'x'; size]), false)
                    .await?;
                left -= size;
            }
            send.send_payload(Bytes::new(), true).await
        },
        read_response(recv),
    )
    .await;
    sent?;
    let res = res?;
    let bytes = res
        .trailers
        .get(BYTES_TRAILER)
        .and_then(|v| v.to_str().ok());
    println!(
        "POST /count  {} {:?} ({BYTES_TRAILER}: {})",
        res.status,
        res.body,
        bytes.unwrap_or("?")
    );

    Ok(())
}
