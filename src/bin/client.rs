use std::io::{Write, BufRead, BufReader};
use std::net::TcpStream;

use color_eyre::eyre::Result;

fn main() -> Result<()> {
    color_eyre::install()?;

    let mut stream = TcpStream::connect("127.0.0.1:6379")?;

    println!("sending PING");
    stream.write_all(b"PING\r\n")?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;

    println!("Response: {line}");

    Ok(())
}
