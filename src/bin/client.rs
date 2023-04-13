use std::io::{BufReader, Write};
use std::net::TcpStream;

use color_eyre::eyre::{Context, Result};
use redis_clone::resp::Message;

fn main() -> Result<()> {
    color_eyre::install()?;

    let mut stream = TcpStream::connect("127.0.0.1:6379")?;
    let mut reader = BufReader::new(stream.try_clone().wrap_err("failed to clone stream")?);

    let messages = vec![
        Message::Array(vec![Message::BulkString(Some(b"PING".to_vec()))]),
        Message::Array(vec![Message::BulkString(Some(b"nonsense".to_vec()))]),
        Message::Array(vec![
            Message::BulkString(Some(b"SET".to_vec())),
            Message::BulkString(Some(b"mykey".to_vec())),
            Message::BulkString(Some(b"hello".to_vec())),
        ]),
        Message::Array(vec![
            Message::BulkString(Some(b"GET".to_vec())),
            Message::BulkString(Some(b"mykey".to_vec())),
        ]),
    ];

    for message in messages {
        println!("sending {:?}", message);
        message.serialize_resp(&mut stream)?;
        stream.flush()?;
        let response = Message::parse_resp(&mut reader)?;
        println!("Response: {response:?}");
    }

    Ok(())
}
