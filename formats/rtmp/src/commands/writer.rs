use amf::{self, Value as AmfValue, amf0::Value as Amf0Value, amf3::Value as Amf3Value};
use std::{collections::HashMap, io};

use super::{
    CallCommandRequest, CallCommandResponse, ConnectCommandRequest, ConnectCommandResponse,
    CreateStreamCommandRequest, CreateStreamCommandResponse, DeleteStreamCommand, OnStatusCommand,
    PauseCommand, Play2Command, PlayCommand, PublishCommand, ReceiveAudioCommand,
    ReceiveVideoCommand, RtmpC2SCommands, RtmpS2CCommands, SeekCommand,
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
            RtmpC2SCommands::Call(command) => self.write_c2s_call_command(command),
            RtmpC2SCommands::CreateStream(command) => self.write_c2s_create_stream_command(command),
            RtmpC2SCommands::Play(command) => self.write_c2s_play_command(command),
            RtmpC2SCommands::Play2(command) => self.write_c2s_play2_command(command),
            RtmpC2SCommands::DeleteStream(command) => self.write_c2s_delete_stream_command(command),
            RtmpC2SCommands::ReceiveAudio(command) => self.write_c2s_receive_audio_command(command),
            RtmpC2SCommands::ReceiveVideo(command) => self.write_c2s_receive_video_command(command),
            RtmpC2SCommands::Publish(command) => self.write_c2s_publish_command(command),
            RtmpC2SCommands::Seek(command) => self.write_c2s_seek_command(command),
            RtmpC2SCommands::Pause(command) => self.write_c2s_pause_command(command),
        }
    }

    pub fn write_s2c_command(&mut self, command: RtmpS2CCommands) -> CommandMessageResult<()> {
        match command {
            RtmpS2CCommands::Connect(command) => self.write_s2c_connect_command(command),
            RtmpS2CCommands::CreateStream(command) => self.write_s2c_create_stream_command(command),
            RtmpS2CCommands::Call(command) => self.write_s2c_call_command(command),
            RtmpS2CCommands::OnStatus(command) => self.write_s2c_on_status_command(command),
        }
    }

    fn write_c2s_connect_command(
        &mut self,
        command: ConnectCommandRequest,
    ) -> CommandMessageResult<()> {
        self.write_amf_str(c2s_command_names::CONNECT)?;
        self.write_amf_number(1)?;
        self.write_amf_object_or_null(Some(command.command_object))?;
        self.write_amf_object_or_null(command.optional_user_arguments)?;
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
        self.write_amf_str(command_name)?;
        self.write_amf_number(1)?;
        self.write_amf_object_or_null(command.properties)?;
        self.write_amf_object_or_null(Some(command.information))?;
        Ok(())
    }

    fn write_c2s_call_command(&mut self, command: CallCommandRequest) -> CommandMessageResult<()> {
        self.write_amf_str(&command.procedure_name)?;
        self.write_amf_number(command.transaction_id)?;
        self.write_amf_object_or_null(command.command_object)?;
        self.write_amf_object_or_null(command.optional_arguments)?;
        Ok(())
    }

    fn write_s2c_call_command(&mut self, command: CallCommandResponse) -> CommandMessageResult<()> {
        self.write_amf_str(&command.command_name)?;
        self.write_amf_number(command.transaction_id)?;
        self.write_amf_object_or_null(command.command_object)?;
        self.write_amf_object_or_null(command.response)?;
        Ok(())
    }

    fn write_c2s_create_stream_command(
        &mut self,
        command: CreateStreamCommandRequest,
    ) -> CommandMessageResult<()> {
        self.write_amf_str(c2s_command_names::CREATE_STREAM)?;
        self.write_amf_number(command.transaction_id)?;
        self.write_amf_object_or_null(command.command_object)?;
        Ok(())
    }

    fn write_s2c_create_stream_command(
        &mut self,
        command: CreateStreamCommandResponse,
    ) -> CommandMessageResult<()> {
        if command.success {
            self.write_amf_str(s2c_command_names::RESULT)?;
        } else {
            self.write_amf_str(s2c_command_names::ERROR)?;
        };
        self.write_amf_number(command.transaction_id)?;
        self.write_amf_object_or_null(command.command_object)?;
        self.write_amf_number(command.stream_id)?;
        Ok(())
    }

    fn write_s2c_on_status_command(
        &mut self,
        command: OnStatusCommand,
    ) -> CommandMessageResult<()> {
        self.write_amf_str(s2c_command_names::ON_STATUS)?;
        self.write_amf_number(0)?;
        self.write_amf_null()?;
        self.write_amf_object_or_null(Some(command.info_object))?;
        Ok(())
    }

    fn write_c2s_play_command(&mut self, command: PlayCommand) -> CommandMessageResult<()> {
        self.write_amf_str(c2s_command_names::PLAY)?;
        self.write_amf_number(0)?;
        self.write_amf_null()?;
        self.write_amf_str(&command.stream_name)?;
        self.write_amf_number(command.start as f64)?;
        self.write_amf_number(command.duration as f64)?;
        self.write_amf_bool(command.reset)?;
        Ok(())
    }

    fn write_c2s_play2_command(&mut self, command: Play2Command) -> CommandMessageResult<()> {
        self.write_amf_str(c2s_command_names::PLAY2)?;
        self.write_amf_number(0)?;
        self.write_amf_null()?;
        self.write_amf_object_or_null(Some(command.parameters))?;
        Ok(())
    }

    fn write_c2s_delete_stream_command(
        &mut self,
        command: DeleteStreamCommand,
    ) -> CommandMessageResult<()> {
        self.write_amf_str(c2s_command_names::DELETE_STREAM)?;
        self.write_amf_number(0)?;
        self.write_amf_null()?;
        self.write_amf_number(command.stream_id)?;
        Ok(())
    }

    fn write_c2s_receive_audio_command(
        &mut self,
        command: ReceiveAudioCommand,
    ) -> CommandMessageResult<()> {
        self.write_amf_str(c2s_command_names::RECEIVE_AUDIO)?;
        self.write_amf_number(0)?;
        self.write_amf_null()?;
        self.write_amf_bool(command.bool_flag)?;
        Ok(())
    }

    fn write_c2s_receive_video_command(
        &mut self,
        command: ReceiveVideoCommand,
    ) -> CommandMessageResult<()> {
        self.write_amf_str(c2s_command_names::RECEIVE_VIDEO)?;
        self.write_amf_number(0)?;
        self.write_amf_null()?;
        self.write_amf_bool(command.bool_flag)?;
        Ok(())
    }

    fn write_c2s_publish_command(&mut self, command: PublishCommand) -> CommandMessageResult<()> {
        self.write_amf_str(c2s_command_names::PUBLISH)?;
        self.write_amf_number(0)?;
        self.write_amf_null()?;
        self.write_amf_str(&command.publishing_name)?;
        self.write_amf_str(&command.publishing_type)?;
        Ok(())
    }

    fn write_c2s_seek_command(&mut self, command: SeekCommand) -> CommandMessageResult<()> {
        self.write_amf_str(c2s_command_names::SEEK)?;
        self.write_amf_number(0)?;
        self.write_amf_null()?;
        self.write_amf_number(command.milliseconds as f64)?;
        Ok(())
    }

    fn write_c2s_pause_command(&mut self, command: PauseCommand) -> CommandMessageResult<()> {
        self.write_amf_str(c2s_command_names::PAUSE)?;
        self.write_amf_number(0)?;
        self.write_amf_null()?;
        self.write_amf_bool(command.pause_flag)?;
        self.write_amf_number(command.milliseconds as f64)?;
        Ok(())
    }

    fn write_amf_str(&mut self, value: &str) -> CommandMessageResult<()> {
        AmfValue::write_str(value, self.inner.by_ref(), self.amf_version)?;
        Ok(())
    }

    fn write_amf_bool(&mut self, value: bool) -> CommandMessageResult<()> {
        AmfValue::write_bool(value, self.inner.by_ref(), self.amf_version)?;
        Ok(())
    }

    fn write_amf_number<T>(&mut self, value: T) -> CommandMessageResult<()>
    where
        T: Into<f64>,
    {
        AmfValue::write_number(value.into(), self.inner.by_ref(), self.amf_version)?;
        Ok(())
    }

    fn write_amf_object_or_null<T>(&mut self, value: Option<T>) -> CommandMessageResult<()>
    where
        T: Into<HashMap<String, AmfValue>>,
    {
        match value {
            Some(obj) => {
                AmfValue::write_key_value_pairs(obj.into(), self.inner.by_ref(), self.amf_version)?
            }
            None => self.write_amf_null()?,
        }
        Ok(())
    }

    fn write_amf_null(&mut self) -> CommandMessageResult<()> {
        AmfValue::write_null(self.inner.by_ref(), self.amf_version)?;
        Ok(())
    }
}
