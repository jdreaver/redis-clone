//! Core server functionality for redis-clone.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::net::TcpListener;
use std::thread;

use color_eyre::eyre::{eyre, Result, WrapErr};

use crate::command::{Command, CommandResponse, Get, Set};
use crate::resp::Message;
use crate::string::RedisString;

/// A `Server` is a redis-clone server.
#[derive(Debug)]
pub struct Server {}

impl Server {
    pub fn start<A>(addr: A) -> Result<Self>
    where
        A: std::net::ToSocketAddrs,
    {
        let listener = TcpListener::bind(addr).wrap_err_with(|| eyre!("failed to start server"))?;

        println!("Listening on {}", listener.local_addr()?);

        loop {
            // Wait for a client to connect.
            let (mut stream, addr) = listener.accept()?;
            println!("connection received from {addr}");

            // Spawn a thread to handle this client.
            thread::spawn(move || {
                let mut write_stream = stream.try_clone().expect("failed to clone stream");
                let mut writer = BufWriter::new(&mut write_stream);
                let mut reader = BufReader::new(&mut stream);

                if let Err(e) = client_loop(&mut reader, &mut writer) {
                    eprintln!("error in client thread: {e}");
                }
                println!("connection closed for addr {addr}");
            });
        }
    }
}

fn client_loop<R, W>(reader: &mut R, writer: &mut BufWriter<&mut W>) -> Result<()>
where
    R: BufRead,
    W: Write,
{
    // TODO: Don't have a single server core per thread. Have threads send
    // commands to server.
    let mut core = ServerCore::new();

    while let Some(response) = process_next_message(&mut core, reader) {
        let response = response.to_resp();

        println!("sending response: {response:?}");
        response
            .serialize_resp(writer)
            .expect("error in client thread");
        writer.flush()?;
    }

    Ok(())
}

fn process_next_message<R>(core: &mut ServerCore, reader: &mut R) -> Option<CommandResponse>
where
    R: BufRead,
{
    let message = match Message::parse_resp(reader) {
        Ok(Some(m)) => m,
        Ok(None) => {
            return None;
        }
        Err(e) => {
            return Some(CommandResponse::Error(format!(
                "error parsing message: {e}"
            )));
        }
    };
    println!("received message: {message:?}");

    let command = match Command::parse_resp(&message) {
        Ok(c) => c,
        Err(e) => {
            return Some(CommandResponse::Error(format!("error parsing RESP: {e}")));
        }
    };
    println!("parsed command: {command:?}");

    let response = core.process_command(command);
    println!("SERVER STATE: {core:?}");

    Some(response)
}

/// A `ServerCore` is primary command processor of the redis-clone server. It
/// contains the key-value store and the logic for handling commands.
#[derive(Debug)]
struct ServerCore {
    key_value: HashMap<RedisString, RedisString>,
}

impl ServerCore {
    pub fn new() -> Self {
        Self {
            key_value: HashMap::new(),
        }
    }

    pub fn process_command(&mut self, command: Command) -> CommandResponse {
        match command {
            Command::Ping => CommandResponse::Pong,
            Command::Get(Get { key }) => {
                let value = self.key_value.get(&key);
                CommandResponse::BulkString(value.cloned())
            }
            Command::Set(Set { key, value }) => {
                self.key_value.insert(key, value);
                CommandResponse::Ok
            }
            Command::RawCommand(c) => CommandResponse::Error(format!("unknown command: {c:?}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ping() {
        let mut core = ServerCore::new();
        let response = core.process_command(Command::Ping);
        assert_eq!(response, CommandResponse::Pong);
    }

    #[test]
    fn test_set_get() {
        let mut core = ServerCore::new();

        let set_command = Command::Set(Set {
            key: RedisString::from("key"),
            value: RedisString::from("value"),
        });
        let response = core.process_command(set_command);
        assert_eq!(response, CommandResponse::Ok);

        let get_command = Command::Get(Get {
            key: RedisString::from("key"),
        });
        let response = core.process_command(get_command);
        assert_eq!(
            response,
            CommandResponse::BulkString(Some(RedisString::from("value")))
        );
    }
}
