use std::io::BufReader;
use std::net::{TcpListener, TcpStream};
use std::thread;

use color_eyre::eyre::{Context, Result};

use redis_clone::command::{Command, CommandResponse, Get, Set};
use redis_clone::resp::Message;
use redis_clone::string::RedisString;

fn main() -> Result<()> {
    color_eyre::install()?;

    let listener = TcpListener::bind("127.0.0.1:6379")?;
    println!("Listening on {}", listener.local_addr()?);

    loop {
        // Wait for a client to connect.
        let (mut stream, addr) = listener.accept()?;
        println!("connection received from {}", addr);

        // Spawn a thread to handle this client.
        thread::spawn(move || {
            let mut writer = stream.try_clone().expect("failed to clone stream");
            let mut reader = BufReader::new(&mut stream);

            if let Err(e) = client_loop(&mut reader, &mut writer) {
                eprintln!("error in client thread: {}", e);
            }
            println!("connection closed for addr {addr}");
        });
    }
}

fn client_loop(reader: &mut BufReader<&mut TcpStream>, writer: &mut TcpStream) -> Result<()> {
    loop {
        let message = Message::parse_resp(reader).wrap_err("failed to parse message")?;
        println!("received message: {:?}", message);

        let command = Command::parse_resp(&message);
        println!("parsed command: {:?}", command);

        let response = match command {
            Ok(Command::Ping) => CommandResponse::Pong,
            Ok(Command::Get(Get { key })) => {
                CommandResponse::BulkString(Some(RedisString::from(format!("got {key:?}"))))
            }
            Ok(Command::Set(Set { key, value })) => CommandResponse::BulkString(Some(
                RedisString::from(format!("set {key:?} to {value:?}")),
            )),
            Ok(Command::RawCommand(c)) => CommandResponse::Error(format!("unknown command: {c:?}")),
            Err(e) => CommandResponse::Error(format!("error parsing command: {e}")),
        };

        let response = response.to_resp();
        println!("sending response: {:?}", response);
        response
            .serialize_resp(writer)
            .expect("error in client thread: ");
    }
}
