# Redis Clone

Toy code for a [Redis](https://redis.io) clone.

## Running

Start Redis via `redis-server`, or use our server with `cargo run --bin server`.
In another terminal, run `cargo run --bin client`.

## TODO

- Integration tests
  - One server with many clients running simultaneously
- Replication to a read-only redis-clone server
- Persistence
- More interesting key/value data structure besides a Rust `HashMap`
