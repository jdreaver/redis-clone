//! Implements the RESP (REdis Serialization Protocol) protocol. See
//! <https://redis.io/docs/reference/protocol-spec/>.

use std::io::{BufRead, Write};

use color_eyre::eyre::{eyre, Result, WrapErr};

use crate::string::RedisString;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    /// Simple Strings are used to transmit non binary-safe strings with minimal
    /// overhead. They cannot contain a CR or LF character.
    SimpleString(String),

    /// Errors are similar to RESP Simple Strings, but the first character is a
    /// minus '-' character instead of a plus.
    Error(String),

    /// Bulk Strings are used in order to represent a single binary-safe string
    /// up to 512 MB in length.
    BulkString(Option<RedisString>),

    /// Arrays are collections of RESP commands. Notably, arrays are used to
    /// send commands from the client to the Redis server.
    Array(Vec<Message>),
}

impl Message {
    pub fn bulk_string(s: &str) -> Self {
        Self::BulkString(Some(RedisString::from(s)))
    }

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
            Self::BulkString(s) => {
                writer.write_all(b"$")?;
                match s {
                    None => {
                        // Null strings are a bit special
                        writer.write_all(b"-1")?;
                        writer.write_all(b"\r\n")?;
                        return Ok(());
                    }
                    Some(s) => {
                        writer.write_all(s.len().to_string().as_bytes())?;
                        writer.write_all(b"\r\n")?;
                        writer.write_all(s.as_bytes())?;
                        writer.write_all(b"\r\n")?;
                    }
                }
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

    /// Reads data from the given reader and parses it into a `Message`.
    ///
    /// A return value of `Ok(None)` indicates that the reader is empty.
    pub fn parse_resp<R>(reader: &mut R) -> Result<Option<Self>>
    where
        R: BufRead,
    {
        let mut line = String::new();
        reader.read_line(&mut line)?;

        if line.is_empty() {
            return Ok(None);
        }

        let line = strip_trailing_crlf(&line)
            .wrap_err_with(|| eyre!("line didn't end with CRLF: {line:?}"))?;

        let resp = match line.chars().next() {
            Some('+') => Self::SimpleString(line[1..].to_string()),
            Some('-') => Self::Error(line[1..].to_string()),
            Some('$') => {
                let len: i32 = line[1..]
                    .parse::<i32>()
                    .wrap_err("invalid bulk string length")?;

                if len >= 0 {
                    #[allow(clippy::cast_sign_loss)]
                    let mut buf = vec![0; len as usize];
                    reader
                        .read_exact(&mut buf)
                        .wrap_err(eyre!("failed to read into buf"))?;

                    // Ensure trailing CRLF!
                    let mut trailing_crlf = [0; 2];
                    reader
                        .read_exact(&mut trailing_crlf)
                        .wrap_err(eyre!("failed to read trailing CRLF"))?;

                    Self::BulkString(Some(RedisString::from(buf)))
                } else if len == -1 {
                    Self::BulkString(None)
                } else {
                    return Err(eyre!("invalid bulk string length"));
                }
            }
            Some('*') => {
                let num_msgs = line[1..]
                    .parse::<usize>()
                    .wrap_err("could not parse array length")?;
                let mut msgs = Vec::with_capacity(num_msgs);
                for i in 0..num_msgs {
                    let msg = Self::parse_resp(reader)
                        .wrap_err(eyre!("failed to parse array elem {i}"))?
                        .ok_or_else(|| eyre!("empty string at array elem {i}"))?;

                    msgs.push(msg);
                }
                Self::Array(msgs)
            }
            Some(c) => return Err(eyre!("invalid message start: {c}")),
            None => {
                return Err(eyre!(
                    "somehow no char even though we checked for empty string"
                ))
            }
        };

        Ok(Some(resp))
    }
}

fn strip_trailing_crlf(s: &str) -> Result<&str> {
    s.strip_suffix("\r\n")
        .ok_or_else(|| eyre!("string does not end with CRLF"))
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::BufReader;

    #[test]
    fn parse_empty_string() {
        let mut buf = BufReader::new(b"" as &[u8]);
        let msg = Message::parse_resp(&mut buf).unwrap();
        assert_eq!(msg, None);
    }

    fn assert_message_round_trip(msg: Message, expected: &[u8]) {
        let mut buf = Vec::new();
        msg.serialize_resp(&mut buf).unwrap();
        // N.B. Strings give clearer error message
        // assert_eq!(buf, expected);
        assert_eq!(
            String::from_utf8(buf.clone()),
            String::from_utf8(expected.to_vec())
        );
        let msg2 = Message::parse_resp(&mut buf.as_slice()).unwrap();
        assert_eq!(Some(msg), msg2);
    }

    #[test]
    fn simple_string_round_trip() {
        assert_message_round_trip(Message::SimpleString("OK".to_string()), b"+OK\r\n");
    }

    #[test]
    fn error_round_trip() {
        assert_message_round_trip(
            Message::Error("ERROR my error".to_string()),
            b"-ERROR my error\r\n",
        );
    }

    #[test]
    fn bulk_string_round_trip() {
        assert_message_round_trip(Message::BulkString(None), b"$-1\r\n");
        assert_message_round_trip(
            Message::BulkString(Some(RedisString::from("hello"))),
            b"$5\r\nhello\r\n",
        );
        assert_message_round_trip(
            Message::BulkString(Some(RedisString::from("hello\r\nwith\r\nnewline"))),
            b"$20\r\nhello\r\nwith\r\nnewline\r\n",
        );
    }

    #[test]
    fn array_round_trip() {
        assert_message_round_trip(Message::Array(Vec::new()), b"*0\r\n");
        assert_message_round_trip(
            Message::Array(vec![Message::SimpleString("OK".to_string())]),
            b"*1\r\n+OK\r\n",
        );
        assert_message_round_trip(
            Message::Array(vec![
                Message::SimpleString("OK".to_string()),
                Message::SimpleString("blah".to_string()),
            ]),
            b"*2\r\n+OK\r\n+blah\r\n",
        );

        assert_message_round_trip(
            Message::Array(vec![
                Message::Array(vec![Message::SimpleString("nested".to_string())]),
                Message::SimpleString("OK".to_string()),
                Message::BulkString(Some(RedisString::from("hello\r\nwith\r\nnewline"))),
                Message::SimpleString("blah".to_string()),
            ]),
            b"*4\r\n*1\r\n+nested\r\n+OK\r\n$20\r\nhello\r\nwith\r\nnewline\r\n+blah\r\n",
        );
    }
}
