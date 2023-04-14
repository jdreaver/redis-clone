//! Core server functionality for redis-clone.

use std::collections::HashMap;
use std::io::{BufReader, BufWriter, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

use color_eyre::eyre::{eyre, Result, WrapErr};
use crossbeam_channel::{Receiver, Sender};

use crate::command::{Command, CommandResponse, Get, Set};
use crate::resp::Message;
use crate::string::RedisString;

/// A `Server` is a redis-clone server.
///
/// It contains a single core worker thread that processes commands and stores
/// data. Each client connection is handled by a separate thread that
/// communicates with the core worker thread via channels.
#[derive(Debug)]
pub struct Server {
    next_thread_id: ThreadId,

    /// Used for child threads to register their response channels so the core
    /// worker thread knows where to send responses.
    response_channels: Arc<Mutex<HashMap<ThreadId, Sender<CommandResponse>>>>,

    /// Used for sending commands to the core worker thread.
    command_sender: Sender<(ThreadId, Command)>,

    /// Used for the core worker thread to receive commands for processing.
    command_receiver: Receiver<(ThreadId, Command)>,
}

type ThreadId = usize;

impl Server {
    pub fn new() -> Self {
        let (command_sender, command_receiver) =
            crossbeam_channel::unbounded::<(ThreadId, Command)>();
        Self {
            next_thread_id: 0,
            response_channels: Arc::new(Mutex::new(HashMap::new())),
            command_sender,
            command_receiver,
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
        self.start_core_worker_thread();

        let listener = TcpListener::bind(addr).wrap_err_with(|| eyre!("failed to start server"))?;
        println!("Listening on {}", listener.local_addr()?);

        loop {
            self.start_next_client_thread(&listener)?;
        }
    }

    fn start_core_worker_thread(&mut self) {
        let command_receiver = self.command_receiver.clone();
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

        // TODO - handle shutdown
    }

    fn start_next_client_thread(&mut self, listener: &TcpListener) -> Result<()> {
        // Wait for a client to connect.
        let (stream, addr) = listener.accept()?;
        println!("connection received from {addr}");

        // Create thread ID and channel for this client.
        let (response_sender, response_receiver) =
            crossbeam_channel::unbounded::<CommandResponse>();
        let thread_id = self.get_thread_id();
        {
            // New scope to ensure lock is released before we spawn the thread.
            self.response_channels
                .lock()
                .map_err(|_| {
                    eyre!("lock was poisoned during a previous access and can no longer be locked")
                })?
                .insert(thread_id, response_sender);
        }

        let mut client_thread = ClientThread::new(
            thread_id,
            addr.to_string(),
            self.command_sender.clone(),
            response_receiver,
            stream,
        );
        thread::spawn(move || client_thread.run_loop());

        Ok(())
    }
}

#[derive(Debug)]
struct ClientThread {
    thread_id: ThreadId,
    client_addr: String,
    command_sender: Sender<(ThreadId, Command)>,
    response_receiver: Receiver<CommandResponse>,
    writer: BufWriter<TcpStream>,
    reader: BufReader<TcpStream>,
}

impl ClientThread {
    fn new(
        thread_id: ThreadId,
        client_addr: String,
        command_sender: Sender<(ThreadId, Command)>,
        response_receiver: Receiver<CommandResponse>,
        stream: TcpStream,
    ) -> Self {
        let write_stream = stream.try_clone().expect("failed to clone stream");
        let writer = BufWriter::new(write_stream);
        let reader = BufReader::new(stream);
        Self {
            thread_id,
            client_addr,
            command_sender,
            response_receiver,
            writer,
            reader,
        }
    }

    fn run_loop(&mut self) {
        if let Err(e) = self.loop_iteration() {
            eprintln!("error in client thread: {e}");
        }
        println!("connection closed for addr {}", self.client_addr);
    }

    fn loop_iteration(&mut self) -> Result<()> {
        while let Some(response) = self.process_next_message() {
            let response = response.to_resp();

            println!("sending response: {response:?}");
            response
                .serialize_resp(&mut self.writer)
                .expect("error in client thread");
            self.writer.flush()?;
        }

        Ok(())
    }

    fn process_next_message(&mut self) -> Option<CommandResponse> {
        let message = match Message::parse_resp(&mut self.reader) {
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
        self.command_sender
            .send((self.thread_id, command))
            .expect("failed to send command");
        let response = self
            .response_receiver
            .recv()
            .expect("failed to receive response");

        Some(response)
    }
}

/// A `ServerCore` is primary command processor of the redis-clone server. It
/// contains the key-value store and the logic for handling commands.
#[derive(Debug)]
struct ServerCore {
    key_value: HashMap<RedisString, RedisString>,
}

impl ServerCore {
    fn new() -> Self {
        Self {
            key_value: HashMap::new(),
        }
    }

    fn process_command(&mut self, command: Command) -> CommandResponse {
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
