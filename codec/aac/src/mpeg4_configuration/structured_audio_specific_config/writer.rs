use bitstream_io::BitWrite;
use num::ToPrimitive;
use utils::traits::writer::BitwiseWriteTo;

use crate::errors::AACCodecError;

use super::{
    DataWithLength, StructuredAudioSpecificConfig, StructuredAudioSpecificConfigChunk, SymbolName,
    SymbolTable,
    midi_file::MidiFile,
    orch_file::{OrchToken, OrchTokenContent},
    sample::{Sample, SampleLoop},
    sbf::Sbf,
    score_file::{
        ControlEvent, Event, EventTime, InstrEvent, ScoreFile, ScoreLine, TableEvent,
        TableEventContent,
    },
    write_data_with_length,
};

impl<W: BitWrite> BitwiseWriteTo<W> for StructuredAudioSpecificConfig {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        for (i, item) in self.chunks.iter().enumerate() {
            writer.write::<3, u8>(item.chunk_type())?;
            item.write_to(writer)?;
            writer.write_bit(i != self.chunks.len())?;
        }
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for OrchToken {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write::<8, u8>(self.content.get_token())?;
        self.content.write_to(writer)?;
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for OrchTokenContent {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        match self {
            Self::Sym(sym) => {
                writer.write::<16, u16>(*sym)?;
                Ok(())
            }
            Self::ValF32(f32) => {
                writer.write::<32, u32>(u32::from_be_bytes(f32.to_be_bytes()))?;
                Ok(())
            }
            Self::ValU32(val) => {
                writer.write::<32, u32>(*val)?;
                Ok(())
            }
            Self::String(DataWithLength { length: _, data }) => {
                writer.write::<8, u8>(data.len().to_u8().unwrap())?;
                writer.write_bytes(data)?;
                Ok(())
            }
            Self::ValU8(val) => {
                writer.write::<8, u8>(*val)?;
                Ok(())
            }
            Self::End => Ok(()),
        }
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for EventTime {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_bit(self.use_if_late)?;
        writer.write::<32, u32>(u32::from_be_bytes(self.time.to_be_bytes()))?;
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for ScoreLine {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        if let Some(time) = self.event_time.as_ref() {
            writer.write_bit(true)?;
            time.write_to(writer)?;
        } else {
            writer.write_bit(false)?;
        }
        writer.write_bit(self.high_priority)?;
        writer.write::<3, u8>(self.event.get_type())?;
        self.event.write_to(writer)?;
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for Event {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        match self {
            Self::Inst(inst) => inst.write_to(writer),
            Self::Control(control) => control.write_to(writer),
            Self::Table(table) => table.write_to(writer),
            Self::End() => Ok(()),
            Self::Tempo { tempo } => {
                writer.write::<32, u32>(u32::from_be_bytes(tempo.to_be_bytes()))?;
                Ok(())
            }
        }
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for TableEventContent {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write::<8, u8>(self.tgen)?;
        if let Some(table_sym) = self.table_sym {
            writer.write_bit(true)?;
            writer.write::<16, u16>(table_sym)?;
        } else {
            writer.write_bit(false)?;
        }
        writer.write::<16, u16>(self.num_pf)?;
        if self.tgen == 0x7D {
            writer.write::<32, u32>(u32::from_be_bytes(self.size.unwrap().to_be_bytes()))?;
            self.ft
                .as_ref()
                .unwrap()
                .iter()
                .try_for_each(|item| writer.write::<16, u16>(*item))?;
        } else {
            self.pf.as_ref().unwrap().iter().try_for_each(|item| {
                writer.write::<32, u32>(u32::from_be_bytes(item.to_be_bytes()))
            })?;
        }
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for TableEvent {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write::<16, u16>(self.tname)?;
        if let Some(content) = self.content.as_ref() {
            writer.write_bit(false)?;
            content.write_to(writer)?;
        } else {
            writer.write_bit(true)?;
        }
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for ControlEvent {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        if let Some(label) = self.label {
            writer.write_bit(true)?;
            writer.write::<16, u16>(label)?;
        } else {
            writer.write_bit(false)?;
        }
        writer.write::<16, u16>(self.varsym)?;
        writer.write::<32, u32>(u32::from_be_bytes(self.value.to_be_bytes()))?;
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for InstrEvent {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        if let Some(label) = self.label {
            writer.write_bit(true)?;
            writer.write::<16, u16>(label)?;
        } else {
            writer.write_bit(false)?;
        }
        writer.write::<16, u16>(self.iname_sym)?;
        writer.write::<32, u32>(u32::from_be_bytes(self.dur.to_be_bytes()))?;
        writer.write::<8, u8>(self.pf.len().to_u8().unwrap())?;
        self.pf
            .iter()
            .try_for_each(|item| writer.write::<32, u32>(u32::from_be_bytes(item.to_be_bytes())))?;
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for ScoreFile {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        write_data_with_length(self, 20, writer)
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for MidiFile {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write::<32, u32>(self.data.len().to_u32().unwrap())?;
        writer.write_bytes(&self.data)?;
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for SampleLoop {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write::<24, u32>(self.loopstart)?;
        writer.write::<24, u32>(self.loopend)?;
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for Sample {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write::<16, u16>(self.sample_name_sym)?;
        writer.write::<24, u32>(self.length)?;
        if let Some(srate) = self.srate {
            writer.write_bit(true)?;
            writer.write::<17, u32>(srate)?;
        } else {
            writer.write_bit(false)?;
        }
        if let Some(lp) = self.sample_loop.as_ref() {
            writer.write_bit(true)?;
            lp.write_to(writer)?;
        } else {
            writer.write_bit(false)?;
        }
        if let Some(base) = self.basecps {
            writer.write_bit(true)?;
            writer.write::<32, u32>(u32::from_be_bytes(base.to_be_bytes()))?;
        } else {
            writer.write_bit(false)?;
        }
        writer.write_bit(self.float_sample)?;
        if self.float_sample {
            self.float_sample_data
                .as_ref()
                .unwrap()
                .iter()
                .try_for_each(|item| {
                    writer.write::<32, u32>(u32::from_be_bytes(item.to_be_bytes()))
                })?;
        } else {
            self.sample_data
                .as_ref()
                .unwrap()
                .iter()
                .try_for_each(|item| writer.write::<16, i16>(*item))?;
        }
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for Sbf {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write::<32, u32>(self.length)?;
        self.data
            .iter()
            .try_for_each(|item| writer.write::<8, i8>(*item))?;
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for SymbolName {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write::<4, u8>(self.length)?;
        writer.write_bytes(&self.name)?;
        Ok(())
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for SymbolTable {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        write_data_with_length(self, 16, writer)
    }
}

impl<W: BitWrite> BitwiseWriteTo<W> for StructuredAudioSpecificConfigChunk {
    type Error = AACCodecError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        match self {
            Self::Orc(orc) => orc.write_to(writer),
            Self::Score(score) => score.write_to(writer),
            Self::SMF(smf) => smf.write_to(writer),
            Self::Samp(samp) => samp.write_to(writer),
            Self::SampleBank(sb) => sb.write_to(writer),
            Self::Sym(sym) => sym.write_to(writer),
        }
    }
}
