//! Implements the RESP (REdis Serialization Protocol) protocol. See
//! <https://redis.io/docs/reference/protocol-spec/>.

use std::io::Write;

use color_eyre::eyre::{eyre, Result};

#[derive(Debug, PartialEq, Eq)]
pub enum Message {
    /// Simple Strings are used to transmit non binary-safe strings with minimal
    /// overhead. They cannot contain a CR or LF character.
    SimpleString(String),
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
            }
        }

        writer.write_all(b"\r\n")?;
        Ok(())
    }

    // TODO: Commands can be multiple lines. We should pass in a BufReader or
    // something in here.
    pub fn parse_resp_line(line: &str) -> Result<Self> {
        match line.chars().next() {
            Some('+') => Ok(Self::SimpleString(line[1..].to_string())),
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
        Message::SimpleString("OK".to_string()).serialize_resp(&mut buf).unwrap();
        assert_eq!(buf, b"+OK\r\n");
    }

    #[test]
    fn simple_parse_resp_line() {
        let msg = Message::parse_resp_line("+OK").unwrap();
        assert_eq!(msg, Message::SimpleString("OK".to_string()));
    }
}
