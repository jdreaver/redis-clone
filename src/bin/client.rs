use std::io::{BufReader, BufWriter, Write};
use std::net::TcpStream;

use color_eyre::eyre::{eyre, Context, Result};

use redis_clone::command::{Command, CommandResponse, Get, Set};
use redis_clone::resp::Message;
use redis_clone::string::RedisString;

fn main() -> Result<()> {
    color_eyre::install()?;

    let stream = TcpStream::connect("127.0.0.1:6379")?;
    let mut write_stream = stream.try_clone().expect("failed to clone stream");
    let mut writer = BufWriter::new(&mut write_stream);
    let mut reader = BufReader::new(stream.try_clone().wrap_err("failed to clone stream")?);

    let commands = vec![
        Command::Ping,
        Command::RawCommand(vec![Message::bulk_string("nonsense")]),
        Command::Set(Set {
            key: RedisString::from("mykey"),
            value: RedisString::from("hello"),
        }),
        Command::Get(Get {
            key: RedisString::from("mykey"),
        }),
    ];

    for command in commands {
        println!("Command:  {:?}", command);
        let message = command.to_resp();
        message.serialize_resp(&mut writer)?;
        writer.flush()?;
        let response = Message::parse_resp(&mut reader)
            .wrap_err(eyre!("failed to parse response"))?
            .ok_or(eyre!("response was empty"))?;
        let response = CommandResponse::parse_resp(response.clone())
            .wrap_err(eyre!("failed to parse {response:?}"))?;
        println!("Response: {response:?}");
    }

    Ok(())
}
