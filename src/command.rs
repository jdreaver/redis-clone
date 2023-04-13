//! Implements Redis commands. See <https://redis.io/commands/>

use crate::resp::Message;

use color_eyre::eyre::{eyre, Result, WrapErr};

/// A `Command` is a well-formed Redis command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Ping,

    /// `RawCommand` is a command that is not supported by this library.
    RawCommand(Vec<Message>),
}

impl Command {
    pub fn to_resp(&self) -> Message {
        let args = match self {
            Self::Ping => vec![Message::bulk_string("PING".to_string())],
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
                String::from_utf8(cmd_str.clone()).wrap_err("command name must be valid UTF-8")?
            }
            _ => return Err(eyre!("command name must be bulk or simple string")),
        };

        match cmd_str.to_uppercase().as_str() {
            "PING" => expect_no_args(Self::Ping, "PING", args),
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
    BulkString(Option<Vec<u8>>),
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
        assert_command_round_trip(&Command::Ping, &[Message::bulk_string("PING".to_string())]);
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
