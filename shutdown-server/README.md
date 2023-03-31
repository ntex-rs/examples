# shutdown-server

Demonstrates how to shutdown the web server in a couple of ways:

1. remotely, via http request

2. sending a SIGINT signal to the server (control-c)
	- ntex server natively supports SIGINT


## Usage

### Running The Server

```bash
cargo run --bin shutdown-server

# Starting 8 workers
# Starting "ntex-service-127.0.0.1:8080" service on 127.0.0.1:8080
```

### Available Routes

- [GET /hello](http://localhost:8080/hello)
  - Regular hello world route
- [POST /stop](http://localhost:8080/stop)
  - Calling this will shutdown the server and exit
