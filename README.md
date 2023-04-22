# Redis Clone

Toy code for a [Redis](https://redis.io) clone. The only neat feature really is
a thread per client connection so IO is multiplexed, but a single, central
worker thread to process commands. The central worker thread doesn't need
locking so it is very fast.

## Running

Start Redis via `redis-server`, or use our server with `cargo run --bin server`.
In another terminal, run `cargo run --bin client`.

Server:

```
$ RUST_BACKTRACE=1 cargo run --bin server --release
2023-04-15T15:12:31.053Z INFO  [redis_clone::server] Listening on 127.0.0.1:6379
2023-04-15T15:12:37.413Z INFO  [redis_clone::server] connection received from 127.0.0.1:37974
2023-04-15T15:12:37.414Z INFO  [redis_clone::server] received message: Array([BulkString(Some("PING"))])
2023-04-15T15:12:37.414Z INFO  [redis_clone::server] parsed command: Ping
2023-04-15T15:12:37.414Z INFO  [redis_clone::server] core thread got command: [0] Ping
2023-04-15T15:12:37.414Z INFO  [redis_clone::server] core thread response: [0] Pong
2023-04-15T15:12:37.414Z INFO  [redis_clone::server] sending response: SimpleString("PONG")
2023-04-15T15:12:37.415Z INFO  [redis_clone::server] received message: Array([BulkString(Some("nonsense"))])
2023-04-15T15:12:37.415Z INFO  [redis_clone::server] sending response: Error("error parsing RESP: unknown command: nonsense")
2023-04-15T15:12:37.415Z INFO  [redis_clone::server] received message: Array([BulkString(Some("SET")), BulkString(Some("mykey")), BulkString(Some("hello"))])
2023-04-15T15:12:37.415Z INFO  [redis_clone::server] parsed command: Set(Set { key: "mykey", value: "hello" })
2023-04-15T15:12:37.415Z INFO  [redis_clone::server] core thread got command: [0] Set(Set { key: "mykey", value: "hello" })
2023-04-15T15:12:37.415Z INFO  [redis_clone::server] core thread response: [0] Ok
2023-04-15T15:12:37.415Z INFO  [redis_clone::server] sending response: SimpleString("OK")
2023-04-15T15:12:37.415Z INFO  [redis_clone::server] received message: Array([BulkString(Some("GET")), BulkString(Some("mykey"))])
2023-04-15T15:12:37.415Z INFO  [redis_clone::server] parsed command: Get(Get { key: "mykey" })
2023-04-15T15:12:37.415Z INFO  [redis_clone::server] core thread got command: [0] Get(Get { key: "mykey" })
2023-04-15T15:12:37.415Z INFO  [redis_clone::server] core thread response: [0] BulkString(Some("hello"))
2023-04-15T15:12:37.415Z INFO  [redis_clone::server] sending response: BulkString(Some("hello"))
2023-04-15T15:12:37.415Z INFO  [redis_clone::server] connection closed for addr 127.0.0.1:37974
```

Client

```
$ RUST_BACKTRACE=1 cargo run --bin client --release
2023-04-15T15:12:37.413Z INFO  [client] Command:  Ping
2023-04-15T15:12:37.415Z INFO  [client] Response: Pong
2023-04-15T15:12:37.415Z INFO  [client] Command:  RawCommand([BulkString(Some("nonsense"))])
2023-04-15T15:12:37.415Z INFO  [client] Response: Error("error parsing RESP: unknown command: nonsense")
2023-04-15T15:12:37.415Z INFO  [client] Command:  Set(Set { key: "mykey", value: "hello" })
2023-04-15T15:12:37.415Z INFO  [client] Response: Ok
2023-04-15T15:12:37.415Z INFO  [client] Command:  Get(Get { key: "mykey" })
2023-04-15T15:12:37.415Z INFO  [client] Response: BulkString(Some("hello"))
```

## TODO

- Integration tests
  - One server with many clients running simultaneously
- Replication to a read-only redis-clone server
- Persistence
- More interesting key/value data structure besides a Rust `HashMap`
