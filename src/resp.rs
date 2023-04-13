//! Implements the RESP (REdis Serialization Protocol) protocol. See
//! <https://redis.io/docs/reference/protocol-spec/>.

use std::io::{BufRead, Write};

use color_eyre::eyre::{eyre, Result, WrapErr};

#[derive(Debug, PartialEq, Eq)]
pub enum Message {
    /// Simple Strings are used to transmit non binary-safe strings with minimal
    /// overhead. They cannot contain a CR or LF character.
    SimpleString(String),

    /// Errors are similar to RESP Simple Strings, but the first character is a
    /// minus '-' character instead of a plus.
    Error(String),
}

impl Message {
    pub fn serialize_resp<W>(&self, writer: &mut W) -> Result<()>
    where
        W: Write,
    {
        match self {
            Self::SimpleString(s) => {
                writer.write_all(b"+")?;
                writer.write_all(s.as_bytes())?;
                writer.write_all(b"\r\n")?;
            }
            Self::Error(s) => {
                writer.write_all(b"-")?;
                writer.write_all(s.as_bytes())?;
                writer.write_all(b"\r\n")?;
            }
        }

        Ok(())
    }

    pub fn parse_resp<R>(reader: &mut R) -> Result<Self>
    where
        R: BufRead,
    {
        let mut lines = reader.lines();

        let first_line = match lines.next() {
            Some(Ok(line)) => line,
            Some(Err(e)) => return Err(e).wrap_err("error reading line"),
            None => return Err(eyre!("no line in message")),
        };

        match first_line.chars().next() {
            Some('+') => Ok(Self::SimpleString(first_line[1..].to_string())),
            Some('-') => Ok(Self::Error(first_line[1..].to_string())),
            Some(c) => Err(eyre!("invalid message start: {c}")),
            None => Err(eyre!("empty message")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_serialize_to_resp() {
        let mut buf = Vec::new();
        Message::SimpleString("OK".to_string())
            .serialize_resp(&mut buf)
            .unwrap();
        assert_eq!(buf, b"+OK\r\n");

        let mut buf = Vec::new();
        Message::Error("ERROR my error".to_string())
            .serialize_resp(&mut buf)
            .unwrap();
        assert_eq!(buf, b"-ERROR my error\r\n");
    }

    #[test]
    fn simple_parse_resp() {
        let input = "+OK".to_string();
        let msg = Message::parse_resp(&mut input.as_bytes()).unwrap();
        assert_eq!(msg, Message::SimpleString("OK".to_string()));

        let input = "-ERROR my error".to_string();
        let msg = Message::parse_resp(&mut input.as_bytes()).unwrap();
        assert_eq!(msg, Message::Error("ERROR my error".to_string()));
    }
}
