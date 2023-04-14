use std::io::{BufRead, BufReader, BufWriter, Write};
use std::net::TcpListener;
use std::thread;

use color_eyre::eyre::Result;

use redis_clone::command::{Command, CommandResponse};
use redis_clone::resp::Message;
use redis_clone::server::Server;

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
            let mut write_stream = stream.try_clone().expect("failed to clone stream");
            let mut writer = BufWriter::new(&mut write_stream);
            let mut reader = BufReader::new(&mut stream);

            if let Err(e) = client_loop(&mut reader, &mut writer) {
                eprintln!("error in client thread: {}", e);
            }
            println!("connection closed for addr {addr}");
        });
    }
}

fn client_loop<R, W>(reader: &mut R, writer: &mut BufWriter<&mut W>) -> Result<()>
where
    R: BufRead,
    W: Write,
{
    // TODO: Don't have a single server per thread. Have threads send
    // commands to server.
    let mut server = Server::new();

    while let Some(response) = process_next_message(&mut server, reader) {
        let response = response.to_resp();

        println!("sending response: {:?}", response);
        response
            .serialize_resp(writer)
            .expect("error in client thread");
        writer.flush()?;
    }

    Ok(())
}

fn process_next_message<R>(server: &mut Server, reader: &mut R) -> Option<CommandResponse>
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
                "error parsing message: {}",
                e
            )));
        }
    };
    println!("received message: {:?}", message);

    let command = match Command::parse_resp(&message) {
        Ok(c) => c,
        Err(e) => {
            return Some(CommandResponse::Error(format!("error parsing RESP: {}", e)));
        }
    };
    println!("parsed command: {:?}", command);

    let response = match server.process_command(command) {
        Ok(r) => r,
        Err(e) => {
            return Some(CommandResponse::Error(format!(
                "error processing command: {}",
                e
            )));
        }
    };

    println!("SERVER STATE: {:?}", server);

    Some(response)
}
