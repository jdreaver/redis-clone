//! Wrapper type for Redis strings. See <https://redis.io/docs/data-types/strings/>.

use std::fmt;

/// A Redis string. This is a wrapper around a `Vec<u8>` that implements `Debug`
/// in a way that tries to print the string as UTF-8 if possible, and otherwise
/// prints the raw bytes. Also provides convenience `From` implementations.
#[derive(Clone, PartialEq, Eq)]
pub struct RedisString(Vec<u8>);

// This custom Debug impl is the main reason this type exists.
impl fmt::Debug for RedisString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", String::from_utf8_lossy(&self.0))
    }
}

impl RedisString {
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl From<Vec<u8>> for RedisString {
    fn from(v: Vec<u8>) -> Self {
        Self(v)
    }
}

impl From<&[u8]> for RedisString {
    fn from(v: &[u8]) -> Self {
        Self(v.to_vec())
    }
}

impl AsRef<[u8]> for RedisString {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<&str> for RedisString {
    fn from(s: &str) -> Self {
        Self(s.as_bytes().to_vec())
    }
}

impl From<String> for RedisString {
    fn from(s: String) -> Self {
        Self(s.as_bytes().to_vec())
    }
}

impl From<RedisString> for Vec<u8> {
    fn from(s: RedisString) -> Self {
        s.0
    }
}

impl TryFrom<RedisString> for String {
    type Error = std::string::FromUtf8Error;

    fn try_from(s: RedisString) -> Result<Self, Self::Error> {
        Self::from_utf8(s.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug() {
        let s = RedisString::from("hello");
        assert_eq!(format!("{s:?}"), "\"hello\"");

        let s = RedisString::from(vec![b'h', b'i', 0xFF, 0x00]);
        assert_eq!(format!("{s:?}"), "\"hi�\\0\"");
    }
}
