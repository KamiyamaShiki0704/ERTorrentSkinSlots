pub const MAGIC: [u8; 4] = *b"ERSS";
pub const PROTOCOL_VERSION: u16 = 1;
pub const WIRE_SIZE: usize = 24;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageKind {
    Hello = 1,
    State = 2,
}

impl TryFrom<u8> for MessageKind {
    type Error = DecodeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Hello),
            2 => Ok(Self::State),
            _ => Err(DecodeError::UnknownKind(value)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WireMessage {
    pub kind: MessageKind,
    pub catalog_hash: u64,
    pub sequence: u32,
    pub selector: u8,
}

impl WireMessage {
    pub fn encode(self) -> [u8; WIRE_SIZE] {
        let mut bytes = [0u8; WIRE_SIZE];
        bytes[0..4].copy_from_slice(&MAGIC);
        bytes[4..6].copy_from_slice(&PROTOCOL_VERSION.to_le_bytes());
        bytes[6] = self.kind as u8;
        bytes[8..16].copy_from_slice(&self.catalog_hash.to_le_bytes());
        bytes[16..20].copy_from_slice(&self.sequence.to_le_bytes());
        bytes[20] = self.selector;
        bytes
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        if bytes.len() != WIRE_SIZE {
            return Err(DecodeError::WrongLength(bytes.len()));
        }
        if bytes[0..4] != MAGIC {
            return Err(DecodeError::WrongMagic);
        }

        let version = u16::from_le_bytes([bytes[4], bytes[5]]);
        if version != PROTOCOL_VERSION {
            return Err(DecodeError::UnsupportedVersion(version));
        }
        if bytes[7] != 0 || bytes[21..24] != [0, 0, 0] {
            return Err(DecodeError::NonZeroReserved);
        }

        Ok(Self {
            kind: MessageKind::try_from(bytes[6])?,
            catalog_hash: u64::from_le_bytes(bytes[8..16].try_into().unwrap()),
            sequence: u32::from_le_bytes(bytes[16..20].try_into().unwrap()),
            selector: bytes[20],
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeError {
    WrongLength(usize),
    WrongMagic,
    UnsupportedVersion(u16),
    UnknownKind(u8),
    NonZeroReserved,
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WrongLength(value) => write!(f, "wrong packet length {value}"),
            Self::WrongMagic => write!(f, "wrong packet magic"),
            Self::UnsupportedVersion(value) => write!(f, "unsupported protocol version {value}"),
            Self::UnknownKind(value) => write!(f, "unknown message kind {value}"),
            Self::NonZeroReserved => write!(f, "reserved packet bytes are non-zero"),
        }
    }
}

impl std::error::Error for DecodeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_round_trips() {
        let message = WireMessage {
            kind: MessageKind::State,
            catalog_hash: 0x1234_5678_9ABC_DEF0,
            sequence: 42,
            selector: 6,
        };
        assert_eq!(WireMessage::decode(&message.encode()).unwrap(), message);
    }

    #[test]
    fn rejects_unknown_version_and_reserved_bytes() {
        let mut bytes = WireMessage {
            kind: MessageKind::Hello,
            catalog_hash: 1,
            sequence: 0,
            selector: 0,
        }
        .encode();
        bytes[4..6].copy_from_slice(&99u16.to_le_bytes());
        assert_eq!(
            WireMessage::decode(&bytes),
            Err(DecodeError::UnsupportedVersion(99))
        );

        bytes[4..6].copy_from_slice(&PROTOCOL_VERSION.to_le_bytes());
        bytes[23] = 1;
        assert_eq!(
            WireMessage::decode(&bytes),
            Err(DecodeError::NonZeroReserved)
        );
    }
}
