use std::io::{BufReader, Write};
use std::net::TcpStream;

use color_eyre::eyre::{Context, Result};
use redis_clone::resp::Message;

fn main() -> Result<()> {
    color_eyre::install()?;

    let mut stream = TcpStream::connect("127.0.0.1:6379")?;
    let mut reader = BufReader::new(stream.try_clone().wrap_err("failed to clone stream")?);

    let messages = vec!["PING", "nonsense"];

    for message in messages {
        // TODO: Send ping using Message. I think we need to send
        // "*1\r\n\$4\r\nPING\r\n" or "*1\r\n+PING\r\n"
        println!("sending {}", message);
        stream.write_all(message.as_bytes())?;
        stream.write_all(b"\r\n")?;
        stream.flush()?;
        let response = Message::parse_resp(&mut reader)?;
        println!("Response: {response:?}");
    }

    Ok(())
}
