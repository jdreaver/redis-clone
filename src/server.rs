//! Core server functionality for redis-clone.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

use color_eyre::eyre::{eyre, Result, WrapErr};
use crossbeam_channel::{Receiver, Sender};

use crate::command::{Command, CommandResponse, Get, Set};
use crate::resp::Message;
use crate::string::RedisString;

/// A `Server` is a redis-clone server.
#[derive(Debug)]
pub struct Server {
    next_thread_id: ThreadId,
    response_channels: Arc<Mutex<HashMap<ThreadId, Sender<CommandResponse>>>>,
}

type ThreadId = usize;

impl Server {
    pub fn new() -> Self {
        Self {
            next_thread_id: 0,
            response_channels: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn get_thread_id(&mut self) -> ThreadId {
        let id = self.next_thread_id;
        self.next_thread_id += 1;
        id
    }

    pub fn start<A>(&mut self, addr: A) -> Result<()>
    where
        A: std::net::ToSocketAddrs,
    {
        // Start core worker thread.
        let (command_sender, command_receiver) =
            crossbeam_channel::unbounded::<(ThreadId, Command)>();
        let core_response_channels = self.response_channels.clone();
        thread::spawn(move || {
            let mut core = ServerCore::new();
            while let Ok((thread_id, command)) = command_receiver.recv() {
                println!("core thread got command: [{thread_id}] {command:?}");
                let response = core.process_command(command);
                println!("core thread response: [{thread_id}] {response:?}");
                core_response_channels
                    .lock()
                    .expect("couldn't lock response channels")
                    .get(&thread_id)
                    .expect("no response channel for thread")
                    .send(response)
                    .expect("failed to send response");
            }
        });

        let listener = TcpListener::bind(addr).wrap_err_with(|| eyre!("failed to start server"))?;

        println!("Listening on {}", listener.local_addr()?);

        loop {
            // Wait for a client to connect.
            let (mut stream, addr) = listener.accept()?;
            println!("connection received from {addr}");

            // Create thread ID and channel for this client.
            let mut command_sender = command_sender.clone();
            let (response_sender, mut response_receiver) =
                crossbeam_channel::unbounded::<CommandResponse>();
            let thread_id = self.get_thread_id();
            self.response_channels
                .lock()
                .expect("couldn't lock response channels")
                .insert(thread_id, response_sender);

            thread::spawn(move || {
                let mut write_stream = stream.try_clone().expect("failed to clone stream");
                let mut writer = BufWriter::new(&mut write_stream);
                let mut reader = BufReader::new(&mut stream);

                if let Err(e) = client_loop(
                    thread_id,
                    &mut reader,
                    &mut writer,
                    &mut command_sender,
                    &mut response_receiver,
                ) {
                    eprintln!("error in client thread: {e}");
                }
                println!("connection closed for addr {addr}");
            });
        }
    }
}

fn client_loop<R, W>(
    thread_id: ThreadId,
    reader: &mut R,
    writer: &mut BufWriter<&mut W>,
    send_command: &mut Sender<(ThreadId, Command)>,
    recv_response: &mut Receiver<CommandResponse>,
) -> Result<()>
where
    R: BufRead,
    W: Write,
{
    while let Some(response) = process_next_message(thread_id, reader, send_command, recv_response)
    {
        let response = response.to_resp();

        println!("sending response: {response:?}");
        response
            .serialize_resp(writer)
            .expect("error in client thread");
        writer.flush()?;
    }

    Ok(())
}

fn process_next_message<R>(
    thread_id: ThreadId,
    reader: &mut R,
    send_command: &mut Sender<(ThreadId, Command)>,
    recv_response: &mut Receiver<CommandResponse>,
) -> Option<CommandResponse>
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

    // Send command off to core, and await the response.
    send_command
        .send((thread_id, command))
        .expect("failed to send command");
    let response = recv_response.recv().expect("failed to receive response");

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
