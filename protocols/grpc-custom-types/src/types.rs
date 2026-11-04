//! Rust types for message fields, wired in by `generate.sh` with `--map`.
//!
//! A mapped field is read and written through the type's [`NativeType`]
//! implementation, so the type decides how the value looks on the wire and
//! which values it accepts.
use std::fmt;

use ntex_grpc::{BytePages, ByteString, Bytes, DecodeError, NativeType, WireType};
use uuid::Uuid;

/// A request id, sent as a 16-byte `bytes` field.
///
/// The generated code needs `Default`, `PartialEq` and `Debug`. The nil
/// UUID is the default value, so it is left out of the encoded message like
/// an empty `bytes` field.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct RequestId(Uuid);

impl RequestId {
    /// A new random id.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// The UUID of this id.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl NativeType for RequestId {
    const TYPE: WireType = WireType::LengthDelimited;

    fn value_len(&self) -> usize {
        16
    }

    /// `src` holds exactly the field value, the length prefix is already
    /// removed.
    fn merge(&mut self, src: &mut Bytes) -> Result<(), DecodeError> {
        let id =
            Uuid::from_slice(src).map_err(|_| DecodeError::new("request id must be 16 bytes"))?;
        *self = Self(id);
        Ok(())
    }

    fn encode_value(&self, dst: &mut BytePages) {
        dst.extend_from_slice(self.0.as_bytes());
    }

    fn is_default(&self) -> bool {
        self.0.is_nil()
    }
}

/// A stock keeping unit such as `BOOK-42`, sent as a `string` field.
///
/// Only 1 to 32 upper-case ASCII letters, digits and `-` are valid, so a
/// decoded message never holds anything else.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Default)]
pub struct Sku(ByteString);

impl Sku {
    /// The longest valid SKU.
    pub const MAX_LEN: usize = 32;

    /// The SKU as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Sku {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A SKU with characters other than `A-Z`, `0-9` and `-`, or of the wrong length.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct InvalidSku;

impl fmt::Display for InvalidSku {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SKU must be 1 to {} characters of A-Z, 0-9 and '-'",
            Sku::MAX_LEN
        )
    }
}

impl std::error::Error for InvalidSku {}

impl TryFrom<ByteString> for Sku {
    type Error = InvalidSku;

    fn try_from(s: ByteString) -> Result<Self, InvalidSku> {
        let valid = (1..=Self::MAX_LEN).contains(&s.len())
            && s.bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'-');
        if valid { Ok(Self(s)) } else { Err(InvalidSku) }
    }
}

impl TryFrom<&str> for Sku {
    type Error = InvalidSku;

    fn try_from(s: &str) -> Result<Self, InvalidSku> {
        Self::try_from(ByteString::from(s))
    }
}

/// Delegates the wire format to `ByteString`, which also checks UTF-8, and
/// adds the SKU rules on top.
impl NativeType for Sku {
    const TYPE: WireType = WireType::LengthDelimited;

    fn value_len(&self) -> usize {
        self.0.value_len()
    }

    fn merge(&mut self, src: &mut Bytes) -> Result<(), DecodeError> {
        let mut s = ByteString::default();
        s.merge(src)?;
        *self = Self::try_from(s).map_err(|e| DecodeError::new(e.to_string()))?;
        Ok(())
    }

    fn encode_value(&self, dst: &mut BytePages) {
        self.0.encode_value(dst);
    }

    fn is_default(&self) -> bool {
        self.0.is_empty()
    }
}
