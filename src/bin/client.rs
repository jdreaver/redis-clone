use std::io::{Write, BufRead, BufReader};
use std::net::TcpStream;

use color_eyre::eyre::Result;
use redis_clone::resp::Message;

fn main() -> Result<()> {
    color_eyre::install()?;

    let mut stream = TcpStream::connect("127.0.0.1:6379")?;

    println!("sending PING");
    // TODO: Send ping using Message. I think we need to send
    // "*1\r\n\$4\r\nPING\r\n" or "*1\r\n+PING\r\n"
    stream.write_all(b"PING\r\n")?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;

    let response = Message::parse_resp_line(&line)?;
    println!("Response: {response:?}");

    Ok(())
}
