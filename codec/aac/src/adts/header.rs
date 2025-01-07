use std::io;

use bitstream_io::{BigEndian, BitReader, BitWriter};

use crate::errors::AacResult;

use super::{fixed_header::FixedHeader, variable_header::VariableHeader};

#[derive(Debug)]
pub struct AdtsHeader {
    pub fixed_header: FixedHeader,
    pub variable_header: VariableHeader,
}

impl AdtsHeader {
    pub fn read_from<R: io::Read>(reader: &mut BitReader<R, BigEndian>) -> AacResult<Self> {
        let fixed_header = FixedHeader::read_from(reader)?;
        let variable_header = VariableHeader::read_from(reader)?;
        Ok(Self {
            fixed_header,
            variable_header,
        })
    }

    pub fn write_to<W: io::Write>(&self, writer: &mut BitWriter<W, BigEndian>) -> AacResult<()> {
        self.fixed_header.write_to(writer)?;
        self.variable_header.write_to(writer)?;
        Ok(())
    }
}
