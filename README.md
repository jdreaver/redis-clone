# Redis Clone

Toy code for a [Redis](https://redis.io) clone.

## Running

Start Redis via `redis-server`, or use our server with `cargo run --bin server`.
In another terminal, run `cargo run --bin client`.

Server:

```
$ RUST_BACKTRACE=1 cargo run --bin server --release
Listening on 127.0.0.1:6379
connection received from 127.0.0.1:47384
received message: Array([BulkString(Some("PING"))])
parsed command: Ping
core thread got command: [0] Ping
core thread response: [0] Pong
sending response: SimpleString("PONG")
received message: Array([BulkString(Some("nonsense"))])
sending response: Error("error parsing RESP: unknown command: nonsense")
received message: Array([BulkString(Some("SET")), BulkString(Some("mykey")), BulkString(Some("hello"))])
parsed command: Set(Set { key: "mykey", value: "hello" })
core thread got command: [0] Set(Set { key: "mykey", value: "hello" })
core thread response: [0] Ok
sending response: SimpleString("OK")
received message: Array([BulkString(Some("GET")), BulkString(Some("mykey"))])
parsed command: Get(Get { key: "mykey" })
core thread got command: [0] Get(Get { key: "mykey" })
core thread response: [0] BulkString(Some("hello"))
sending response: BulkString(Some("hello"))
connection closed for addr 127.0.0.1:47384
```

Client

```
$ RUST_BACKTRACE=1 cargo run --bin client --release
Command:  Ping
Response: Pong
Command:  RawCommand([BulkString(Some("nonsense"))])
Response: Error("error parsing RESP: unknown command: nonsense")
Command:  Set(Set { key: "mykey", value: "hello" })
Response: Ok
Command:  Get(Get { key: "mykey" })
Response: BulkString(Some("hello"))
```

## TODO

- Integration tests
  - One server with many clients running simultaneously
- Replication to a read-only redis-clone server
- Persistence
- More interesting key/value data structure besides a Rust `HashMap`
