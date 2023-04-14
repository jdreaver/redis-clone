//! Core server functionality for redis-clone.

use std::collections::HashMap;

use color_eyre::eyre::Result;

use crate::command::{Command, CommandResponse, Get, Set};
use crate::string::RedisString;

/// A `Server` is the core of the redis-clone server. It contains the
/// key-value store and the logic for handling commands.
#[derive(Debug)]
pub struct Server {
    key_value: HashMap<RedisString, RedisString>,
}

impl Server {
    pub fn new() -> Self {
        Self {
            key_value: HashMap::new(),
        }
    }

    pub fn process_command(&mut self, command: Command) -> Result<CommandResponse> {
        match command {
            Command::Ping => Ok(CommandResponse::Pong),
            Command::Get(Get { key }) => {
                let value = self.key_value.get(&key);
                Ok(CommandResponse::BulkString(value.cloned()))
            }
            Command::Set(Set { key, value }) => {
                self.key_value.insert(key, value);
                Ok(CommandResponse::Ok)
            }
            Command::RawCommand(c) => Ok(CommandResponse::Error(format!("unknown command: {c:?}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ping() {
        let mut server = Server::new();
        let response = server.process_command(Command::Ping).unwrap();
        assert_eq!(response, CommandResponse::Pong);
    }

    #[test]
    fn test_set_get() {
        let mut server = Server::new();

        let set_command = Command::Set(Set {
            key: RedisString::from("key"),
            value: RedisString::from("value"),
        });
        let response = server.process_command(set_command).unwrap();
        assert_eq!(response, CommandResponse::Ok);

        let get_command = Command::Get(Get {
            key: RedisString::from("key"),
        });
        let response = server.process_command(get_command).unwrap();
        assert_eq!(
            response,
            CommandResponse::BulkString(Some(RedisString::from("value")))
        );
    }
}
