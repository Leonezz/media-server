use std::io;

pub trait WriteTo<W: io::Write>: Sized {
    type Error;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error>;
}
