//! Calls `helloworld.Greeter/SayHello` and prints the reply.
//!
//! Usage: `grpc-helloworld-client [NAME]`
use std::time::Duration;

use grpc_helloworld::{ADDR, connect, helloworld::HelloRequest};

#[ntex::main]
async fn main() {
    let _ = env_logger::try_init();
    let name = std::env::args().nth(1).unwrap_or_default();

    let client = connect(ADDR);
    let msg = HelloRequest { name: name.into() };
    let mut req = client.say_hello(&msg);
    req.timeout(Duration::from_secs(3));

    match req.send().await {
        Ok(res) => println!("{}", res.message),
        Err(err) => {
            eprintln!("call failed with {:?}: {err}", err.status());
            std::process::exit(1);
        }
    }
}
