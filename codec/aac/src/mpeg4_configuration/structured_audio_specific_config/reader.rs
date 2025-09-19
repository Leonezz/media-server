use bitstream_io::BitRead;
use utils::traits::reader::BitwiseReadFrom;

use crate::errors::AACCodecError;

use super::{
    DataWithLength, StructuredAudioSpecificConfig, StructuredAudioSpecificConfigChunk, SymbolName,
    SymbolTable,
    midi_file::MidiFile,
    orch_file::{OrchFile, OrchToken, OrchTokenContent},
    read_data_with_length,
    sample::{Sample, SampleLoop},
    sbf::Sbf,
    score_file::{
        ControlEvent, Event, EventTime, InstrEvent, ScoreFile, ScoreLine, TableEvent,
        TableEventContent,
    },
};

impl<R: BitRead> BitwiseReadFrom<R> for OrchToken {
    type Error = AACCodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let mut done = 0;
        let token = reader.read::<8, u8>()?;
        let content = match token {
            0xF0 => OrchTokenContent::Sym(reader.read::<16, u16>()?),
            0xF1 => OrchTokenContent::ValF32(f32::from_be_bytes(
                reader.read::<32, u32>()?.to_be_bytes(),
            )),
            0xF2 => OrchTokenContent::ValU32(reader.read::<32, u32>()?),
            0xF3 => {
                let length = reader.read::<8, u8>()?;
                let data = {
                    let mut data = vec![];
                    for _ in 0..length {
                        data.push(reader.read::<8, u8>()?);
                    }
                    data
                };
                OrchTokenContent::String(DataWithLength { length, data })
            }
            0xF4 => OrchTokenContent::ValU8(reader.read::<8, u8>()?),
            0xFF => {
                done = 1;
                OrchTokenContent::End
            }
            _ => return Err(AACCodecError::UnknownOrchToken(token)),
        };
        Ok(Self {
            done,
            token,
            content,
        })
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for OrchFile {
    type Error = AACCodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let length = reader.read::<16, u16>()?;
        let data = {
            let mut data = vec![];
            for _ in 0..length {
                data.push(OrchToken::read_from(reader)?);
            }
            data
        };
        Ok(Self { length, data })
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for ScoreFile {
    type Error = AACCodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let num_lines = reader.read::<20, u32>()?;
        read_data_with_length(num_lines, reader)
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for EventTime {
    type Error = AACCodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let use_if_late = reader.read_bit()?;
        let time = f32::from_be_bytes(reader.read::<32, u32>()?.to_be_bytes());
        Ok(Self { use_if_late, time })
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for InstrEvent {
    type Error = AACCodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let has_label = reader.read_bit()?;
        let label = if has_label {
            Some(reader.read::<16, u16>()?)
        } else {
            None
        };
        let iname_sym = reader.read::<16, u16>()?;
        let dur = f32::from_be_bytes(reader.read::<32, u32>()?.to_be_bytes());
        let num_pf = reader.read::<8, u8>()?;
        let pf = {
            let mut pf = vec![];
            for _ in 0..num_pf {
                pf.push(f32::from_be_bytes(reader.read::<32, u32>()?.to_be_bytes()));
            }
            pf
        };
        Ok(Self {
            has_label,
            label,
            iname_sym,
            dur,
            num_pf,
            pf,
        })
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for ControlEvent {
    type Error = AACCodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let has_label = reader.read_bit()?;
        let label = if has_label {
            Some(reader.read::<16, u16>()?)
        } else {
            None
        };
        let varsym = reader.read::<16, u16>()?;
        let value = f32::from_be_bytes(reader.read::<32, u32>()?.to_be_bytes());
        Ok(Self {
            has_label,
            label,
            varsym,
            value,
        })
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for TableEventContent {
    type Error = AACCodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let tgen = reader.read::<8, u8>()?; // TODO: check this token type
        let refers_to_sample = reader.read_bit()?;
        let table_sym = if refers_to_sample {
            Some(reader.read::<16, u16>()?)
        } else {
            None
        };
        let num_pf = reader.read::<16, u16>()?;
        let (size, ft, pf) = if tgen == 0x7D {
            let size = f32::from_be_bytes(reader.read::<32, u32>()?.to_be_bytes());
            let mut ft = vec![];
            for _ in 0..num_pf.checked_sub(1).unwrap() {
                ft.push(reader.read::<16, u16>()?);
            }
            (Some(size), Some(ft), None)
        } else {
            let mut pf = vec![];
            for _ in 0..num_pf {
                pf.push(f32::from_be_bytes(reader.read::<32, u32>()?.to_be_bytes()));
            }
            (None, None, Some(pf))
        };
        Ok(Self {
            tgen,
            refers_to_sample,
            table_sym,
            num_pf,
            size,
            ft,
            pf,
        })
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for TableEvent {
    type Error = AACCodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let tname = reader.read::<16, u16>()?;
        let destroy = reader.read_bit()?;
        let content = if !destroy {
            Some(TableEventContent::read_from(reader)?)
        } else {
            None
        };
        Ok(Self {
            tname,
            destroy,
            content,
        })
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for ScoreLine {
    type Error = AACCodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let has_time = reader.read_bit()?;
        let event_time = if has_time {
            Some(EventTime::read_from(reader)?)
        } else {
            None
        };
        let high_priority = reader.read_bit()?;
        let event_type = reader.read::<3, u8>()?;
        let event = match event_type {
            0b000 => Event::Inst(InstrEvent::read_from(reader)?),
            0b001 => Event::Control(ControlEvent::read_from(reader)?),
            0b010 => Event::Table(TableEvent::read_from(reader)?),
            0b100 => Event::End(),
            0b101 => Event::Tempo {
                tempo: f32::from_be_bytes(reader.read::<32, u32>()?.to_be_bytes()),
            },
            _ => return Err(AACCodecError::UnknownScoreLineType(event_type)),
        };
        Ok(Self {
            has_time,
            event_time,
            high_priority,
            event_type,
            event,
        })
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for MidiFile {
    type Error = AACCodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let length = reader.read::<32, u32>()?;
        let data = {
            let mut data = vec![];
            for _ in 0..length {
                data.push(reader.read::<8, u8>()?);
            }
            data
        };
        Ok(Self { length, data })
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for SampleLoop {
    type Error = AACCodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let loopstart = reader.read::<24, u32>()?;
        let loopend = reader.read::<24, u32>()?;
        Ok(Self { loopstart, loopend })
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for Sample {
    type Error = AACCodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let sample_name_sym = reader.read::<16, u16>()?;
        let length = reader.read::<24, u32>()?;
        let has_srate = reader.read_bit()?;
        let srate = if has_srate {
            Some(reader.read::<17, u32>()?)
        } else {
            None
        };
        let has_loop = reader.read_bit()?;
        let sample_loop = if has_loop {
            Some(SampleLoop::read_from(reader)?)
        } else {
            None
        };
        let has_base = reader.read_bit()?;
        let basecps = if has_base {
            Some(f32::from_be_bytes(reader.read::<32, u32>()?.to_be_bytes()))
        } else {
            None
        };
        let float_sample = reader.read_bit()?;
        let (float_sample_data, sample_data) = if float_sample {
            let mut data = vec![];
            for _ in 0..length {
                data.push(f32::from_be_bytes(reader.read::<32, u32>()?.to_be_bytes()));
            }
            (Some(data), None)
        } else {
            let mut data = vec![];
            for _ in 0..length {
                data.push(reader.read::<16, i16>()?);
            }
            (None, Some(data))
        };
        Ok(Sample {
            sample_name_sym,
            length,
            has_srate,
            srate,
            has_loop,
            sample_loop,
            has_base,
            basecps,
            float_sample,
            float_sample_data,
            sample_data,
        })
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for Sbf {
    type Error = AACCodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let length = reader.read::<32, u32>()?;
        let data = {
            let mut data = vec![];
            for _ in 0..length {
                data.push(reader.read::<8, i8>()?);
            }
            data
        };
        Ok(Self { length, data })
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for SymbolName {
    type Error = AACCodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let length = reader.read::<4, u8>()?;
        let name = {
            let mut data = vec![];
            for _ in 0..length {
                data.push(reader.read::<8, u8>()?);
            }
            data
        };
        Ok(Self { length, name })
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for SymbolTable {
    type Error = AACCodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let length = reader.read::<16, u16>()?;
        read_data_with_length(length, reader)
    }
}

impl<R: BitRead> BitwiseReadFrom<R> for StructuredAudioSpecificConfig {
    type Error = AACCodecError;
    fn read_from(reader: &mut R) -> Result<Self, Self::Error> {
        let mut chunks = vec![];
        loop {
            let chunk_type = reader.read::<3, u8>()?;
            match chunk_type {
                0b000 => {
                    chunks.push(StructuredAudioSpecificConfigChunk::Orc(
                        OrchFile::read_from(reader)?,
                    ));
                }
                0b001 => {
                    chunks.push(StructuredAudioSpecificConfigChunk::Score(
                        ScoreFile::read_from(reader)?,
                    ));
                }
                0b010 => {
                    chunks.push(StructuredAudioSpecificConfigChunk::SMF(
                        MidiFile::read_from(reader)?,
                    ));
                }
                0b011 => {
                    chunks.push(StructuredAudioSpecificConfigChunk::Samp(Sample::read_from(
                        reader,
                    )?));
                }
                0b100 => {
                    chunks.push(StructuredAudioSpecificConfigChunk::SampleBank(
                        Sbf::read_from(reader)?,
                    ));
                }
                0b101 => chunks.push(StructuredAudioSpecificConfigChunk::Sym(
                    SymbolTable::read_from(reader)?,
                )),
                _ => unreachable!(),
            };
            let more_data = reader.read_bit()?;
            if !more_data {
                break;
            }
        }
        Ok(Self { chunks })
    }
}
