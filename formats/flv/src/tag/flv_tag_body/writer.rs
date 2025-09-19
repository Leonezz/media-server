use std::io;

use tokio_util::either::Either;
use utils::traits::writer::WriteTo;

use crate::errors::FLVError;

use super::Filter;

impl<W: io::Write> WriteTo<W> for Filter {
    type Error = FLVError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        self.encryption_header.write_to(writer)?;
        match &self.filter_params.filter_params {
            Either::Left(encryption_params) => {
                encryption_params.write_to(writer)?;
            }
            Either::Right(se_params) => {
                se_params.write_to(writer)?;
            }
        }
        Ok(())
    }
}
