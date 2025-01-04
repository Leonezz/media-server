use std::io::{self, Cursor};

pub trait ReadFrom<R: io::Read>: Sized {
    type Error;
    fn read_from(reader: R) -> Result<Self, Self::Error>;
}

pub trait TryReadFrom<R: AsRef<[u8]>>: Sized {
    type Error;
    fn try_read_from(reader: &mut Cursor<R>) -> Result<Option<Self>, Self::Error>;
}

pub trait ReadRemainingFrom<Header, R: io::Read>: Sized {
    type Error;
    fn read_remaining_from(header: Header, reader: R) -> Result<Self, Self::Error>;
}

pub trait TryReadRemainingFrom<Header, R: AsRef<[u8]>>: Sized {
    type Error;
    fn try_read_remaining_from(
        header: Header,
        reader: &mut Cursor<R>,
    ) -> Result<Option<Self>, Self::Error>;
}
