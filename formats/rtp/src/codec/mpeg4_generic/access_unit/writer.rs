use std::io;

use tokio_util::either::Either;
use utils::traits::writer::WriteTo;

use crate::codec::mpeg4_generic::errors::RtpMpeg4Error;

use super::{AccessUnit, AccessUnitFragment, AccessUnitSection};

impl<W: io::Write> WriteTo<W> for AccessUnit {
    type Error = RtpMpeg4Error;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_all(&self.body)?;
        Ok(())
    }
}

impl<W: io::Write> WriteTo<W> for AccessUnitFragment {
    type Error = RtpMpeg4Error;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_all(&self.body)?;
        Ok(())
    }
}

impl<W: io::Write> WriteTo<W> for AccessUnitSection {
    type Error = RtpMpeg4Error;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        match &self.access_units_or_fragment {
            Either::Left(aus) => aus
                .iter()
                .try_for_each(|item| item.write_to(writer.by_ref())),
            Either::Right(frag) => frag.write_to(writer.by_ref()),
        }
    }
}
