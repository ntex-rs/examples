//! Calls every method of `echo.Echo`, then `echo.Stats/GetStats`.
use std::{error::Error, time::Duration};

use grpc_server::echo::{EchoRequest, StatsRequest};
use grpc_server::{ADDR, COUNT_TRAILER, PREFIX_HEADER, connect};
use ntex::util::join;
use ntex_grpc::{SendStream, StreamError};

#[ntex::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let _ = env_logger::try_init();
    let (echo, stats) = connect(ADDR);

    // unary, with a request header and a deadline
    let msg = echo_request("hello");
    let mut req = echo.unary_echo(&msg);
    req.header(PREFIX_HEADER, "> ")
        .timeout(Duration::from_secs(3));
    println!("UnaryEcho           {}", req.send().await?.message);

    // server streaming
    let msg = EchoRequest {
        repeat: 3,
        ..echo_request("again")
    };
    let mut rx = echo.server_streaming_echo(&msg).open().await?;
    while let Some(reply) = rx.recv().await {
        println!("ServerStreamingEcho {}", reply?.message);
    }

    // client streaming: send while `send()` waits for the reply
    let tx = SendStream::new();
    let (res, sent) = join(echo.client_streaming_echo(&tx).send(), async {
        for word in ["one", "two", "three"] {
            tx.send(&echo_request(word)).await?;
        }
        // the end of the stream, the server answers now
        tx.close();
        Ok::<_, StreamError>(())
    })
    .await;
    sent?;
    let res = res?;
    // extra trailers of the reply
    let count = res
        .trailers()
        .get(COUNT_TRAILER)
        .and_then(|v| v.to_str().ok());
    println!(
        "ClientStreamingEcho {} ({} messages)",
        res.message,
        count.unwrap_or("?")
    );

    // bidirectional streaming: send and receive at the same time
    let (tx, mut rx) = echo.bidi_streaming_echo().open().await?;
    let (sent, received) = join(
        async {
            for word in ["ping", "pong"] {
                tx.send(&echo_request(word)).await?;
            }
            tx.close();
            Ok::<_, StreamError>(())
        },
        async {
            while let Some(reply) = rx.recv().await {
                println!("BidiStreamingEcho   {}", reply?.message);
            }
            Ok::<_, StreamError>(())
        },
    )
    .await;
    sent?;
    received?;

    let res = stats.get_stats(&StatsRequest {}).send().await?;
    println!("GetStats            {} calls", res.calls);
    Ok(())
}

fn echo_request(message: &str) -> EchoRequest {
    EchoRequest {
        message: message.into(),
        ..Default::default()
    }
}
