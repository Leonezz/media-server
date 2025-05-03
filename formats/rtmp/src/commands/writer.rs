use amf_formats;
use num::ToPrimitive;
use std::io;
use tokio_util::either::Either;
use utils::traits::writer::WriteTo;

use crate::chunk::errors::ChunkMessageError;

use super::{
    CallCommandRequest, CallCommandResponse, ConnectCommandRequest, ConnectCommandResponse,
    CreateStreamCommandRequest, CreateStreamCommandResponse, DeleteStreamCommand, OnStatusCommand,
    PauseCommand, Play2Command, PlayCommand, PublishCommand, ReceiveAudioCommand,
    ReceiveVideoCommand, RtmpC2SCommands, RtmpS2CCommands, SeekCommand,
    consts::{c2s_command_names, s2c_command_names},
};

pub struct RtmpCommandWriteWrapper<'a, T>(pub &'a T, pub amf_formats::Version);

impl<'a, T> RtmpCommandWriteWrapper<'a, T> {
    pub fn new(inner: &'a T, version: amf_formats::Version) -> Self {
        Self(inner, version)
    }
}

impl<'a, W: io::Write> WriteTo<W> for RtmpCommandWriteWrapper<'a, RtmpC2SCommands> {
    type Error = ChunkMessageError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        let (value, amf_version) = (self.0, self.1);
        match value {
            RtmpC2SCommands::Connect(command) => {
                RtmpCommandWriteWrapper::new(command, amf_version).write_to(writer)
            }
            RtmpC2SCommands::Call(command) => {
                RtmpCommandWriteWrapper::new(command, amf_version).write_to(writer)
            }
            RtmpC2SCommands::CreateStream(command) => {
                RtmpCommandWriteWrapper::new(command, amf_version).write_to(writer)
            }
            RtmpC2SCommands::Play(command) => {
                RtmpCommandWriteWrapper::new(command, amf_version).write_to(writer)
            }
            RtmpC2SCommands::Play2(command) => {
                RtmpCommandWriteWrapper::new(command, amf_version).write_to(writer)
            }
            RtmpC2SCommands::DeleteStream(command) => {
                RtmpCommandWriteWrapper::new(command, amf_version).write_to(writer)
            }
            RtmpC2SCommands::ReceiveAudio(command) => {
                RtmpCommandWriteWrapper::new(command, amf_version).write_to(writer)
            }
            RtmpC2SCommands::ReceiveVideo(command) => {
                RtmpCommandWriteWrapper::new(command, amf_version).write_to(writer)
            }
            RtmpC2SCommands::Publish(command) => {
                RtmpCommandWriteWrapper::new(command, amf_version).write_to(writer)
            }
            RtmpC2SCommands::Seek(command) => {
                RtmpCommandWriteWrapper::new(command, amf_version).write_to(writer)
            }
            RtmpC2SCommands::Pause(command) => {
                RtmpCommandWriteWrapper::new(command, amf_version).write_to(writer)
            }
        }
    }
}

impl<'a, W: io::Write> WriteTo<W> for RtmpCommandWriteWrapper<'a, ConnectCommandRequest> {
    type Error = ChunkMessageError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        let (value, amf_version) = (self.0, self.1);
        amf_formats::Value::write_str(c2s_command_names::CONNECT, writer, amf_version)?;
        amf_formats::Value::write_number(1, writer, amf_version)?;
        amf_formats::Value::write_nullable_object(
            Some(value.command_object.clone()),
            writer,
            amf_version,
        )?;
        amf_formats::Value::write_nullable_object(
            value.optional_user_arguments.clone(),
            writer,
            amf_version,
        )?;

        Ok(())
    }
}

impl<'a, W: io::Write> WriteTo<W> for RtmpCommandWriteWrapper<'a, CallCommandRequest> {
    type Error = ChunkMessageError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        let (command, version) = (self.0, self.1);
        amf_formats::Value::write_str(&command.procedure_name, writer, version)?;
        amf_formats::Value::write_number(command.transaction_id, writer, version)?;
        amf_formats::Value::write_nullable_object(command.command_object.clone(), writer, version)?;

        if command.optional_arguments.is_some() {
            match command
                .optional_arguments
                .clone()
                .expect("this cannot be none")
            {
                Either::Left(any) => any.write_to(writer)?,
                Either::Right(object) => {
                    amf_formats::Value::write_nullable_object(Some(object), writer, version)?
                }
            }
        }
        Ok(())
    }
}

impl<'a, W: io::Write> WriteTo<W> for RtmpCommandWriteWrapper<'a, CreateStreamCommandRequest> {
    type Error = ChunkMessageError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        let (command, version) = (self.0, self.1);
        amf_formats::Value::write_str(c2s_command_names::CREATE_STREAM, writer, version)?;
        amf_formats::Value::write_number(command.transaction_id, writer, version)?;
        amf_formats::Value::write_nullable_object(command.command_object.clone(), writer, version)?;
        Ok(())
    }
}

impl<'a, W: io::Write> WriteTo<W> for RtmpCommandWriteWrapper<'a, PlayCommand> {
    type Error = ChunkMessageError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        let (command, version) = (self.0, self.1);
        amf_formats::Value::write_str(c2s_command_names::PLAY, writer, version)?;
        amf_formats::Value::write_number(0, writer, version)?;
        amf_formats::Value::write_null(writer, version)?;
        amf_formats::Value::write_str(&command.stream_name, writer, version)?;
        amf_formats::Value::write_number(command.start.to_f64().unwrap(), writer, version)?;
        amf_formats::Value::write_number(command.duration.to_f64().unwrap(), writer, version)?;
        amf_formats::Value::write_bool(command.reset, writer, version)?;
        Ok(())
    }
}

impl<'a, W: io::Write> WriteTo<W> for RtmpCommandWriteWrapper<'a, Play2Command> {
    type Error = ChunkMessageError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        let (command, version) = (self.0, self.1);
        amf_formats::Value::write_str(c2s_command_names::PLAY2, writer, version)?;
        amf_formats::Value::write_number(0, writer, version)?;
        amf_formats::Value::write_null(writer, version)?;
        amf_formats::Value::write_nullable_object(
            Some(command.parameters.clone()),
            writer,
            version,
        )?;
        Ok(())
    }
}

impl<'a, W: io::Write> WriteTo<W> for RtmpCommandWriteWrapper<'a, DeleteStreamCommand> {
    type Error = ChunkMessageError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        let (command, version) = (self.0, self.1);
        amf_formats::Value::write_str(c2s_command_names::DELETE_STREAM, writer, version)?;
        amf_formats::Value::write_number(0, writer, version)?;
        amf_formats::Value::write_null(writer, version)?;
        amf_formats::Value::write_number(command.stream_id, writer, version)?;
        Ok(())
    }
}

impl<'a, W: io::Write> WriteTo<W> for RtmpCommandWriteWrapper<'a, ReceiveAudioCommand> {
    type Error = ChunkMessageError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        let (command, version) = (self.0, self.1);
        amf_formats::Value::write_str(c2s_command_names::RECEIVE_AUDIO, writer, version)?;
        amf_formats::Value::write_number(0, writer, version)?;
        amf_formats::Value::write_null(writer, version)?;
        amf_formats::Value::write_bool(command.bool_flag, writer, version)?;
        Ok(())
    }
}

impl<'a, W: io::Write> WriteTo<W> for RtmpCommandWriteWrapper<'a, ReceiveVideoCommand> {
    type Error = ChunkMessageError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        let (command, version) = (self.0, self.1);
        amf_formats::Value::write_str(c2s_command_names::RECEIVE_VIDEO, writer, version)?;
        amf_formats::Value::write_number(0, writer, version)?;
        amf_formats::Value::write_null(writer, version)?;
        amf_formats::Value::write_bool(command.bool_flag, writer, version)?;
        Ok(())
    }
}

impl<'a, W: io::Write> WriteTo<W> for RtmpCommandWriteWrapper<'a, PublishCommand> {
    type Error = ChunkMessageError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        let (command, version) = (self.0, self.1);
        amf_formats::Value::write_str(c2s_command_names::PUBLISH, writer, version)?;
        amf_formats::Value::write_number(0, writer, version)?;
        amf_formats::Value::write_null(writer, version)?;
        amf_formats::Value::write_str(&command.publishing_name, writer, version)?;
        amf_formats::Value::write_str(&command.publishing_type, writer, version)?;
        Ok(())
    }
}

impl<'a, W: io::Write> WriteTo<W> for RtmpCommandWriteWrapper<'a, SeekCommand> {
    type Error = ChunkMessageError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        let (command, version) = (self.0, self.1);
        amf_formats::Value::write_str(c2s_command_names::SEEK, writer, version)?;
        amf_formats::Value::write_number(0, writer, version)?;
        amf_formats::Value::write_null(writer, version)?;
        amf_formats::Value::write_number(
            command
                .milliseconds
                .to_f64()
                .expect("milliseconds overflow f64"),
            writer,
            version,
        )?;
        Ok(())
    }
}

impl<'a, W: io::Write> WriteTo<W> for RtmpCommandWriteWrapper<'a, PauseCommand> {
    type Error = ChunkMessageError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        let (command, version) = (self.0, self.1);
        amf_formats::Value::write_str(c2s_command_names::PAUSE, writer, version)?;
        amf_formats::Value::write_number(0, writer, version)?;
        amf_formats::Value::write_null(writer, version)?;
        amf_formats::Value::write_bool(command.pause_flag, writer, version)?;
        amf_formats::Value::write_number(
            command
                .milliseconds
                .to_f64()
                .expect("milliseconds overflow f64"),
            writer,
            version,
        )?;
        Ok(())
    }
}

impl<'a, W: io::Write> WriteTo<W> for RtmpCommandWriteWrapper<'a, RtmpS2CCommands> {
    type Error = ChunkMessageError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        let (command, version) = (self.0, self.1);
        match command {
            RtmpS2CCommands::Connect(command) => {
                RtmpCommandWriteWrapper::new(command, version).write_to(writer)
            }
            RtmpS2CCommands::CreateStream(command) => {
                RtmpCommandWriteWrapper::new(command, version).write_to(writer)
            }
            RtmpS2CCommands::Call(command) => {
                RtmpCommandWriteWrapper::new(command, version).write_to(writer)
            }
            RtmpS2CCommands::OnStatus(command) => {
                RtmpCommandWriteWrapper::new(command, version).write_to(writer)
            }
        }
    }
}

impl<'a, W: io::Write> WriteTo<W> for RtmpCommandWriteWrapper<'a, ConnectCommandResponse> {
    type Error = ChunkMessageError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        let (command, version) = (self.0, self.1);
        let command_name = if command.success {
            s2c_command_names::RESULT
        } else {
            s2c_command_names::ERROR
        };
        amf_formats::Value::write_str(command_name, writer, version)?;
        amf_formats::Value::write_number(1, writer, version)?;
        amf_formats::Value::write_nullable_object(command.properties.clone(), writer, version)?;
        if let Some(info) = &command.information {
            match info {
                Either::Left(any) => any.write_to(writer)?,
                Either::Right(object) => amf_formats::Value::write_nullable_object(
                    Some(object.clone()),
                    writer,
                    version,
                )?,
            }
        }
        Ok(())
    }
}

impl<'a, W: io::Write> WriteTo<W> for RtmpCommandWriteWrapper<'a, CreateStreamCommandResponse> {
    type Error = ChunkMessageError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        let (command, version) = (self.0, self.1);
        let command_name = if command.success {
            s2c_command_names::RESULT
        } else {
            s2c_command_names::ERROR
        };
        amf_formats::Value::write_str(command_name, writer, version)?;
        amf_formats::Value::write_number(command.transaction_id, writer, version)?;
        amf_formats::Value::write_nullable_object(command.command_object.clone(), writer, version)?;
        amf_formats::Value::write_number(command.stream_id, writer, version)?;
        Ok(())
    }
}

impl<'a, W: io::Write> WriteTo<W> for RtmpCommandWriteWrapper<'a, CallCommandResponse> {
    type Error = ChunkMessageError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        let (command, version) = (self.0, self.1);
        amf_formats::Value::write_str(&command.command_name, writer, version)?;
        amf_formats::Value::write_number(command.transaction_id, writer, version)?;
        amf_formats::Value::write_nullable_object(command.command_object.clone(), writer, version)?;
        amf_formats::Value::write_nullable_object(command.response.clone(), writer, version)?;
        Ok(())
    }
}

impl<'a, W: io::Write> WriteTo<W> for RtmpCommandWriteWrapper<'a, OnStatusCommand> {
    type Error = ChunkMessageError;
    fn write_to(&self, writer: &mut W) -> Result<(), Self::Error> {
        let (command, version) = (self.0, self.1);
        amf_formats::Value::write_str(s2c_command_names::ON_STATUS, writer, version)?;
        amf_formats::Value::write_number(0, writer, version)?;
        amf_formats::Value::write_null(writer, version)?;
        amf_formats::Value::write_nullable_object(
            Some(command.info_object.clone()),
            writer,
            version,
        )?;
        Ok(())
    }
}
