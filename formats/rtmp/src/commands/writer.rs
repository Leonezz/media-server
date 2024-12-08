use amf::{self, Value as AmfValue, amf0::Value as Amf0Value, amf3::Value as Amf3Value};
use std::{collections::HashMap, io};

use super::{
    ConnectCommandRequest, ConnectCommandResponse, RtmpC2SCommands, RtmpS2CCommands,
    consts::{c2s_command_names, s2c_command_names},
    errors::CommandMessageResult,
};
pub struct Writer<W> {
    inner: W,
    amf_version: amf::Version,
}

impl<W> Writer<W>
where
    W: io::Write,
{
    pub fn new(inner: W, amf_version: amf::Version) -> Self {
        Self { inner, amf_version }
    }

    pub fn write_c2s_command(&mut self, command: RtmpC2SCommands) -> CommandMessageResult<()> {
        match command {
            RtmpC2SCommands::Connect(command) => self.write_c2s_connect_command(command),
            RtmpC2SCommands::Call(command) => todo!(),
            RtmpC2SCommands::CreateStream(command) => todo!(),
            RtmpC2SCommands::Play(command) => todo!(),
            RtmpC2SCommands::Play2(command) => todo!(),
            RtmpC2SCommands::DeleteStream(command) => todo!(),
            RtmpC2SCommands::ReceiveAudio(command) => todo!(),
            RtmpC2SCommands::ReceiveVideo(command) => todo!(),
            RtmpC2SCommands::Publish(command) => todo!(),
            RtmpC2SCommands::Seek(command) => todo!(),
            RtmpC2SCommands::Pause(command) => todo!(),
        }
    }

    pub fn write_s2c_command(&mut self, command: RtmpS2CCommands) -> CommandMessageResult<()> {
        match command {
            RtmpS2CCommands::Connect(command) => self.write_s2c_connect_command(command),
            RtmpS2CCommands::CreateStream(command) => todo!(),
            RtmpS2CCommands::Call(command) => todo!(),
            RtmpS2CCommands::OnStatus(command) => todo!(),
        }
    }

    fn write_c2s_connect_command(
        &mut self,
        command: ConnectCommandRequest,
    ) -> CommandMessageResult<()> {
        AmfValue::write_str(
            c2s_command_names::CONNECT,
            self.inner.by_ref(),
            self.amf_version,
        )?;
        AmfValue::write_number(1.0, self.inner.by_ref(), self.amf_version)?;
        AmfValue::write_key_value_pairs(
            command.command_object.into(),
            self.inner.by_ref(),
            self.amf_version,
        )?;
        AmfValue::write_key_value_pairs(
            command
                .optional_user_arguments
                .unwrap_or(HashMap::default()),
            self.inner.by_ref(),
            self.amf_version,
        )?;
        Ok(())
    }

    fn write_s2c_connect_command(
        &mut self,
        command: ConnectCommandResponse,
    ) -> CommandMessageResult<()> {
        let command_name = if command.success {
            s2c_command_names::RESULT
        } else {
            s2c_command_names::ERROR
        };

        AmfValue::write_str(command_name, self.inner.by_ref(), self.amf_version)?;
        AmfValue::write_number(1.0, self.inner.by_ref(), self.amf_version)?;
        AmfValue::write_key_value_pairs(
            command.properties.unwrap_or(HashMap::new()),
            self.inner.by_ref(),
            self.amf_version,
        )?;
        AmfValue::write_key_value_pairs(
            command.information,
            self.inner.by_ref(),
            self.amf_version,
        )?;
        Ok(())
    }
}
