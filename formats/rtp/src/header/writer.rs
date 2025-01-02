use crate::errors::RtpResult;
use byteorder::{BigEndian, WriteBytesExt};
use std::io;

use super::{RtpHeader, RtpHeaderExtension};

impl RtpHeader {
    pub fn write_to<W>(&self, mut writer: W) -> RtpResult<()>
    where
        W: io::Write,
    {
        let first_byte = ((self.version & 0b11) << 6)
            | ((self.padding as u8) << 5)
            | ((self.extension as u8) << 4)
            | (self.csrc_count & 0b1111);
        writer.write_u8(first_byte)?;
        writer.write_u8(((self.marker as u8) << 7) | (self.payload_type & 0b0111_1111))?;
        writer.write_u16::<BigEndian>(self.sequence_number)?;
        writer.write_u32::<BigEndian>(self.timestamp)?;
        writer.write_u32::<BigEndian>(self.ssrc)?;
        for csrc in &self.csrc_list {
            writer.write_u32::<BigEndian>(csrc.clone())?;
        }

        if let Some(header_extension) = &self.header_extension {
            header_extension.write_to(writer.by_ref())?;
        }

        Ok(())
    }
}

impl RtpHeaderExtension {
    pub fn write_to<W>(&self, mut writer: W) -> RtpResult<()>
    where
        W: io::Write,
    {
        writer.write_u16::<BigEndian>(self.profile_defined)?;
        writer.write_u16::<BigEndian>(self.length)?;
        writer.write_all(&self.bytes)?;
        Ok(())
    }
}
