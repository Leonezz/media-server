use crate::errors::H264CodecError;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromaFormatIdc {
    Monochrome = 0,
    Chroma420 = 1,
    Chroma422 = 2,
    Chroma444 = 3,
}

impl TryFrom<u8> for ChromaFormatIdc {
    type Error = H264CodecError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ChromaFormatIdc::Monochrome),
            1 => Ok(ChromaFormatIdc::Chroma420),
            2 => Ok(ChromaFormatIdc::Chroma422),
            3 => Ok(ChromaFormatIdc::Chroma444),
            _ => Err(Self::Error::UnknownChromaFormatIdc(value)),
        }
    }
}

impl From<ChromaFormatIdc> for u8 {
    fn from(value: ChromaFormatIdc) -> Self {
        match value {
            ChromaFormatIdc::Monochrome => 0,
            ChromaFormatIdc::Chroma420 => 1,
            ChromaFormatIdc::Chroma422 => 2,
            ChromaFormatIdc::Chroma444 => 3,
        }
    }
}
