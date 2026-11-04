//! Calls `helloworld.Greeter/SayHello` over TLS and prints the reply.
//!
//! The client trusts `../../tls/certs/ca.pem` and connects to `localhost`,
//! the name in the server certificate.
//!
//! Usage: `grpc-tls-client [NAME]`

use std::time::Duration;

use grpc_helloworld::helloworld::HelloRequest;
use grpc_tls::{CONNECT_ADDR, client_config, connect};
use tls_x509_example::Error;

#[ntex::main]
async fn main() -> Result<(), Error> {
    let _ = env_logger::try_init();
    let name = std::env::args().nth(1).unwrap_or_default();

    let config = client_config(&std::fs::read("../../tls/certs/ca.pem")?)?;
    let client = connect(CONNECT_ADDR, config);
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
    Ok(())
}
