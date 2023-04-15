# Redis Clone

Toy code for a [Redis](https://redis.io) clone.

## Running

Start Redis via `redis-server`, or use our server with `cargo run --bin server`.
In another terminal, run `cargo run --bin client`.

## TODO

- Use [proptest](https://docs.rs/proptest/latest/proptest/) to assert round trip parsing
  - <https://altsysrq.github.io/proptest-book>
  - Also <https://docs.rs/proptest-derive/latest/proptest_derive/>
- Integration tests
  - One server with many clients running simultaneously
- Replication to a read-only redis-clone server
- Persistence
- More interesting key/value data structure besides a Rust `HashMap`
