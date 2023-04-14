use color_eyre::eyre::Result;

use redis_clone::server::Server;

fn main() -> Result<()> {
    color_eyre::install()?;

    let mut server = Server::new();
    server.start("127.0.0.1:6379")?;

    Ok(())
}
