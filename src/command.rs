//! Implements Redis commands. See <https://redis.io/commands/>

use crate::resp::Message;

use color_eyre::eyre::{eyre, Result, WrapErr};

use crate::string::RedisString;

/// A `Command` is a well-formed Redis command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Ping,
    Get(Get),
    Set(Set),

    /// `RawCommand` is a command that is not supported by this library.
    RawCommand(Vec<Message>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Get {
    pub key: RedisString,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Set {
    pub key: RedisString,
    pub value: RedisString,
}

impl Command {
    pub fn to_resp(&self) -> Message {
        let args = match self {
            Self::Ping => vec![Message::bulk_string("PING")],
            Self::Get(get) => vec![
                Message::bulk_string("GET"),
                Message::BulkString(Some(get.key.clone())),
            ],
            Self::Set(set) => vec![
                Message::bulk_string("SET"),
                Message::BulkString(Some(set.key.clone())),
                Message::BulkString(Some(set.value.clone())),
            ],
            Self::RawCommand(args) => args.clone(),
        };
        Message::Array(args)
    }

    pub fn parse_resp(resp: Message) -> Result<Self> {
        let Message::Array(elems) = resp else { return Err(eyre!("commands must be an array")) };

        let Some((cmd_message, args)) = elems.split_first() else { return Err(eyre!("commands must have at least one element")) };

        let cmd_str: String = match cmd_message {
            Message::SimpleString(cmd_str) => cmd_str.clone(),
            Message::BulkString(Some(cmd_str)) => {
                String::try_from(cmd_str.clone()).wrap_err("command name must be valid UTF-8")?
            }
            _ => return Err(eyre!("command name must be bulk or simple string")),
        };

        match cmd_str.to_uppercase().as_str() {
            "PING" => expect_no_args(Self::Ping, "PING", args),
            "GET" => match args {
                [Message::BulkString(Some(key))] => Ok(Self::Get(Get { key: key.clone() })),
                _ => Err(eyre!("GET must have a single key argument")),
            },
            "SET" => match args {
                [Message::BulkString(Some(key)), Message::BulkString(Some(value))] => {
                    Ok(Self::Set(Set {
                        key: key.clone(),
                        value: value.clone(),
                    }))
                }
                _ => Err(eyre!("SET must have a key and value argument")),
            },
            _ => Err(eyre!("unknown command: {cmd_str}")),
        }
    }
}

/// Helper function to ensure that a command has no arguments.
fn expect_no_args(cmd: Command, cmd_str: &str, args: &[Message]) -> Result<Command> {
    if !args.is_empty() {
        return Err(eyre!("{cmd_str} takes no arguments"));
    }
    Ok(cmd)
}

/// A `CommandResponse` is a valid response to a command from Redis.
#[derive(Debug, PartialEq, Eq)]
pub enum CommandResponse {
    Pong,
    Ok,
    Error(String),
    BulkString(Option<RedisString>),
}

impl CommandResponse {
    pub fn to_resp(&self) -> Message {
        match self {
            Self::Pong => Message::SimpleString("PONG".to_string()),
            Self::Ok => Message::SimpleString("OK".to_string()),
            Self::Error(e) => Message::Error(e.clone()),
            Self::BulkString(s) => Message::BulkString(s.clone()),
        }
    }

    pub fn parse_resp(resp: Message) -> Result<Self> {
        match resp {
            Message::SimpleString(s) => match s.as_str() {
                "PONG" => Ok(Self::Pong),
                "OK" => Ok(Self::Ok),
                _ => Err(eyre!("unknown simple string response: {s}")),
            },
            Message::Error(e) => Ok(Self::Error(e)),
            Message::BulkString(s) => Ok(Self::BulkString(s)),
            Message::Array(_) => Err(eyre!("array response not supported for command responses")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_command_round_trip(cmd: &Command, expected: &[Message]) {
        let expected = Message::Array(expected.to_vec());
        let got = cmd.to_resp();
        assert_eq!(got, expected);
        let cmd2 = Command::parse_resp(got).unwrap();
        assert_eq!(cmd, &cmd2);
    }

    fn assert_command_response_round_trip(response: &CommandResponse, expected: &Message) {
        let got = response.to_resp();
        assert_eq!(&got, expected);
        let response2 = CommandResponse::parse_resp(got).unwrap();
        assert_eq!(response, &response2);
    }

    #[test]
    fn ping_round_trip() {
        assert_command_round_trip(&Command::Ping, &[Message::bulk_string("PING")]);
    }

    #[test]
    fn get_round_trip() {
        let cmd = Command::Get(Get {
            key: RedisString::from("foo"),
        });
        assert_command_round_trip(
            &cmd,
            &[Message::bulk_string("GET"), Message::bulk_string("foo")],
        );
    }

    #[test]
    fn set_round_trip() {
        let cmd = Command::Set(Set {
            key: RedisString::from("foo"),
            value: RedisString::from("bar"),
        });
        assert_command_round_trip(
            &cmd,
            &[
                Message::bulk_string("SET"),
                Message::bulk_string("foo"),
                Message::bulk_string("bar"),
            ],
        );
    }

    #[test]
    fn pong_round_trip() {
        assert_command_response_round_trip(
            &CommandResponse::Pong,
            &Message::SimpleString("PONG".to_string()),
        );
    }

    #[test]
    fn ok_round_trip() {
        assert_command_response_round_trip(
            &CommandResponse::Ok,
            &Message::SimpleString("OK".to_string()),
        );
    }
}
