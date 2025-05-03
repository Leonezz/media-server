// todo!()

use std::io;

use utils::traits::writer::WriteTo;

use super::ex_audio_body::{
    AudioChannel, AudioChannelOrder, AudioMultichannelConfig, make_channel_masks,
};
use crate::errors::FLVError;
use byteorder::{BigEndian, WriteBytesExt};

impl<W: io::Write> WriteTo<W> for AudioMultichannelConfig {
    type Error = FLVError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        writer.write_u8(self.channel_order.into())?;
        writer.write_u8(self.channel_mapping.len() as u8)?;
        match self.channel_order {
            AudioChannelOrder::Unspecified => {}
            AudioChannelOrder::Custom => {
                let buffer: Vec<u8> = self
                    .channel_mapping
                    .iter()
                    .map(|v| <AudioChannel as Into<u8>>::into(*v))
                    .collect();
                writer.write_all(&buffer)?;
            }
            AudioChannelOrder::Native => {
                let mask = make_channel_masks(&self.channel_mapping);
                writer.write_u32::<BigEndian>(mask)?;
            }
        }
        Ok(())
    }
}
