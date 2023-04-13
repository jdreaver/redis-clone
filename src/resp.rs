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

    /// Arrays are collections of RESP commands. Notably, arrays are used to
    /// send commands from the client to the Redis server.
    Array(Vec<Message>),
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
            Self::Array(msgs) => {
                writer.write_all(b"*")?;
                writer.write_all(msgs.len().to_string().as_bytes())?;
                writer.write_all(b"\r\n")?;

                for msg in msgs {
                    msg.serialize_resp(writer)?;
                }
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
            Some('*') => {
                let num_msgs = first_line[1..]
                    .parse::<usize>()
                    .wrap_err("invalid array length")?;
                let mut msgs = Vec::with_capacity(num_msgs);
                for i in 0..num_msgs {
                    msgs.push(
                        Self::parse_resp(reader)
                            .wrap_err(eyre!("failed to parse array elem {i}"))?,
                    );
                }
                Ok(Self::Array(msgs))
            }
            Some(c) => Err(eyre!("invalid message start: {c}")),
            None => Err(eyre!("empty message")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_message_round_trip(msg: &Message, expected: &[u8]) {
        let mut buf = Vec::new();
        msg.serialize_resp(&mut buf).unwrap();
        assert_eq!(buf, expected);
        let msg2 = Message::parse_resp(&mut buf.as_slice()).unwrap();
        assert_eq!(msg, &msg2);
    }

    #[test]
    fn simple_string_round_trip() {
        assert_message_round_trip(&Message::SimpleString("OK".to_string()), b"+OK\r\n");
    }

    #[test]
    fn error_round_trip() {
        assert_message_round_trip(
            &Message::Error("ERROR my error".to_string()),
            b"-ERROR my error\r\n",
        );
    }

    #[test]
    fn array_round_trip() {
        assert_message_round_trip(&Message::Array(Vec::new()), b"*0\r\n");
        assert_message_round_trip(
            &Message::Array(vec![Message::SimpleString("OK".to_string())]),
            b"*1\r\n+OK\r\n",
        );
        assert_message_round_trip(
            &Message::Array(vec![
                Message::SimpleString("OK".to_string()),
                Message::SimpleString("blah".to_string()),
            ]),
            b"*2\r\n+OK\r\n+blah\r\n",
        );

        assert_message_round_trip(
            &Message::Array(vec![
                Message::Array(vec![Message::SimpleString("nested".to_string())]),
                Message::SimpleString("OK".to_string()),
                Message::SimpleString("blah".to_string()),
            ]),
            b"*3\r\n*1\r\n+nested\r\n+OK\r\n+blah\r\n",
        );
    }
}
