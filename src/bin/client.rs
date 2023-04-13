use std::io::{BufReader, Write};
use std::net::TcpStream;

use color_eyre::eyre::{eyre, Context, Result};

use redis_clone::command::{Command, CommandResponse};
use redis_clone::resp::Message;

fn main() -> Result<()> {
    color_eyre::install()?;

    let mut stream = TcpStream::connect("127.0.0.1:6379")?;
    let mut reader = BufReader::new(stream.try_clone().wrap_err("failed to clone stream")?);

    let commands = vec![
        Command::Ping,
        Command::RawCommand(vec![Message::BulkString(Some(b"nonsense".to_vec()))]),
        Command::RawCommand(vec![
            Message::BulkString(Some(b"SET".to_vec())),
            Message::BulkString(Some(b"mykey".to_vec())),
            Message::BulkString(Some(b"hello".to_vec())),
        ]),
        Command::RawCommand(vec![
            Message::BulkString(Some(b"GET".to_vec())),
            Message::BulkString(Some(b"mykey".to_vec())),
        ]),
    ];

    for command in commands {
        println!("Command:  {:?}", command);
        let message = command.to_resp();
        message.serialize_resp(&mut stream)?;
        stream.flush()?;
        let response = Message::parse_resp(&mut reader)?;
        let response = CommandResponse::parse_resp(response.clone())
            .wrap_err(eyre!("failed to parse {response:?}"))?;
        println!("Response: {response:?}");
    }

    Ok(())
}
