use std::io;

pub trait WriteTo<W: io::Write>: Sized {
    type Error;
    fn write_to(&self, writer: W) -> Result<(), Self::Error>;
}
