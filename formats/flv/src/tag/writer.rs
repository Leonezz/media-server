use std::io;
use utils::traits::writer::WriteTo;

use crate::errors::FLVError;

use super::FLVTag;

impl<W: io::Write> WriteTo<W> for FLVTag {
    type Error = FLVError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        self.tag_header.write_to(writer)?;
        self.body_with_filter.write_to(writer)?;
        Ok(())
    }
}
