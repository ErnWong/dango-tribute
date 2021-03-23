use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use crate::packet_reader::PacketReader;

use js_sys::Date;

/// A Timestamp for a moment in time that can be read/written to/from a byte
/// stream
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Timestamp {
    time: u64,
}

impl Timestamp {
    /// Get a Timestamp for the current moment
    pub fn now() -> Self {
        Timestamp {
            time: Date::now() as u64,
        }
    }

    /// Write the Timestamp into an outgoing packet's byte stream
    pub fn write(&self, buffer: &mut Vec<u8>) {
        buffer.write_u64::<BigEndian>(self.time).unwrap();
    }

    /// Read a Timestamp from an incoming packet's byte stream
    pub fn read(reader: &mut PacketReader) -> Self {
        let cursor = reader.get_cursor();
        let time = cursor.read_u64::<BigEndian>().unwrap();

        Timestamp { time }
    }
}
