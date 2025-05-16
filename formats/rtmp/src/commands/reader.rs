use crate::{chunk::errors::ChunkMessageError, commands::consts::s2c_command_names};

use super::{
    CallCommandRequest, CallCommandResponse, ConnectCommandRequest, ConnectCommandRequestObject,
    ConnectCommandResponse, CreateStreamCommandRequest, CreateStreamCommandResponse,
    DeleteStreamCommand, OnStatusCommand, PauseCommand, Play2Command, PlayCommand, PublishCommand,
    ReceiveAudioCommand, ReceiveVideoCommand, RtmpC2SCommands, RtmpS2CCommands,
    RtmpS2CCommandsType, SeekCommand, consts::c2s_command_names,
};

use num::ToPrimitive;
use std::{
    backtrace::Backtrace,
    io::{self},
};
use tokio_util::either::Either;
use utils::traits::reader::ReadRemainingFrom;

impl<R: io::Read> ReadRemainingFrom<amf_formats::Version, R> for RtmpC2SCommands {
    type Error = ChunkMessageError;
    fn read_remaining_from(
        header: amf_formats::Version,
        mut reader: R,
    ) -> Result<Self, Self::Error> {
        let command_name =
            amf_formats::Value::read_string(reader.by_ref(), header)?.ok_or_else(|| {
                ChunkMessageError::UnexpectedAmfType {
                    amf_type: "expect string type".to_owned(),
                    backtrace: Backtrace::capture(),
                }
            })?;

        match command_name.as_str() {
            c2s_command_names::CONNECT => Ok(RtmpC2SCommands::Connect(
                ConnectCommandRequest::read_remaining_from(header, reader)?,
            )),
            c2s_command_names::CLOSE => todo!(), // FIXME no spec on this one
            c2s_command_names::CREATE_STREAM => Ok(RtmpC2SCommands::CreateStream(
                CreateStreamCommandRequest::read_remaining_from(header, reader)?,
            )),
            c2s_command_names::PLAY => Ok(RtmpC2SCommands::Play(PlayCommand::read_remaining_from(
                header, reader,
            )?)),
            c2s_command_names::PLAY2 => Ok(RtmpC2SCommands::Play2(
                Play2Command::read_remaining_from(header, reader)?,
            )),
            c2s_command_names::DELETE_STREAM => Ok(RtmpC2SCommands::DeleteStream(
                DeleteStreamCommand::read_remaining_from(header, reader)?,
            )),
            c2s_command_names::CLOSE_STREAM => todo!(), // FIXME no spec on this one
            c2s_command_names::RECEIVE_AUDIO => Ok(RtmpC2SCommands::ReceiveAudio(
                ReceiveAudioCommand::read_remaining_from(header, reader)?,
            )),
            c2s_command_names::RECEIVE_VIDEO => Ok(RtmpC2SCommands::ReceiveVideo(
                ReceiveVideoCommand::read_remaining_from(header, reader)?,
            )),
            c2s_command_names::PUBLISH => Ok(RtmpC2SCommands::Publish(
                PublishCommand::read_remaining_from(header, reader)?,
            )),
            c2s_command_names::SEEK => Ok(RtmpC2SCommands::Seek(SeekCommand::read_remaining_from(
                header, reader,
            )?)),
            c2s_command_names::PAUSE => Ok(RtmpC2SCommands::Pause(
                PauseCommand::read_remaining_from(header, reader)?,
            )),
            procedure_name => Ok(RtmpC2SCommands::Call(
                CallCommandRequest::read_remaining_from(
                    (header, procedure_name.to_owned()),
                    reader,
                )?,
            )), // call
        }
    }
}

impl<R: io::Read> ReadRemainingFrom<amf_formats::Version, R> for ConnectCommandRequest {
    type Error = ChunkMessageError;
    fn read_remaining_from(
        header: amf_formats::Version,
        mut reader: R,
    ) -> Result<Self, Self::Error> {
        let transaction_id = amf_formats::Value::read_number(reader.by_ref(), header)?
            .ok_or_else(|| ChunkMessageError::UnexpectedAmfType {
                amf_type: "expect number type".to_owned(),
                backtrace: Backtrace::capture(),
            })?
            .to_u8()
            .expect("trasaction id overflow u8");
        if transaction_id != 1 {
            tracing::warn!(
                "connect transaction_id should be 1, got {} instead",
                transaction_id
            );
        }
        let command_object_map = amf_formats::Value::read_object(reader.by_ref(), header)?;
        if command_object_map.is_none() {
            return Err(ChunkMessageError::UnexpectedAmfType {
                amf_type: "expect a key-value pair type".to_string(),
                backtrace: Backtrace::capture(),
            });
        }
        let command_object_map = command_object_map.expect("there must be some value");
        let command_object: ConnectCommandRequestObject = command_object_map.try_into()?;

        let optional_user_arguments =
            amf_formats::Value::read_object(reader, header).unwrap_or(None);
        Ok(ConnectCommandRequest {
            command_name: c2s_command_names::CONNECT.to_string(),
            transaction_id,
            command_object,
            optional_user_arguments,
        })
    }
}

impl<R: io::Read> ReadRemainingFrom<amf_formats::Version, R> for CreateStreamCommandRequest {
    type Error = ChunkMessageError;
    fn read_remaining_from(
        header: amf_formats::Version,
        mut reader: R,
    ) -> Result<Self, Self::Error> {
        let transaction_id =
            amf_formats::Value::read_number(reader.by_ref(), header)?.ok_or_else(|| {
                ChunkMessageError::UnexpectedAmfType {
                    amf_type: "expect number type".to_owned(),
                    backtrace: Backtrace::capture(),
                }
            })?;

        let command_object = amf_formats::Value::read_object(reader, header).unwrap_or(None);
        Ok(CreateStreamCommandRequest {
            command_name: c2s_command_names::CREATE_STREAM.to_string(),
            transaction_id,
            command_object,
        })
    }
}

impl<R: io::Read> ReadRemainingFrom<amf_formats::Version, R> for PlayCommand {
    type Error = ChunkMessageError;
    fn read_remaining_from(
        header: amf_formats::Version,
        mut reader: R,
    ) -> Result<Self, Self::Error> {
        let transaction_id = amf_formats::Value::read_number(reader.by_ref(), header)?
            .ok_or_else(|| ChunkMessageError::UnexpectedAmfType {
                amf_type: "expect number type".to_owned(),
                backtrace: Backtrace::capture(),
            })?
            .to_u8()
            .expect("transaction id overflow u8");
        if transaction_id != 0 {
            tracing::warn!(
                "play transaction_id should be 0, got {} instead",
                transaction_id
            );
        }
        let _ = amf_formats::Value::read_null(reader.by_ref(), header)?.ok_or_else(|| {
            ChunkMessageError::UnexpectedAmfType {
                amf_type: "expect a null type".to_owned(),
                backtrace: Backtrace::capture(),
            }
        });
        let stream_name =
            amf_formats::Value::read_string(reader.by_ref(), header)?.ok_or_else(|| {
                ChunkMessageError::UnexpectedAmfType {
                    amf_type: "expect string type".to_owned(),
                    backtrace: Backtrace::capture(),
                }
            })?;
        let start = amf_formats::Value::read_number(reader.by_ref(), header)?
            .ok_or_else(|| ChunkMessageError::UnexpectedAmfType {
                amf_type: "expect number type".to_owned(),
                backtrace: Backtrace::capture(),
            })?
            .to_i64()
            .expect("overflow i64");
        let duration = amf_formats::Value::read_number(reader.by_ref(), header).unwrap_or(None);
        let reset = amf_formats::Value::read_bool(reader.by_ref(), header).unwrap_or(None);
        Ok(PlayCommand {
            _command_name: c2s_command_names::PLAY.to_string(),
            _transaction_id: transaction_id,
            stream_name,
            start,
            duration: duration.unwrap_or(-1.0).to_i64().unwrap(),
            reset: reset.unwrap_or(false),
        })
    }
}

impl<R: io::Read> ReadRemainingFrom<amf_formats::Version, R> for Play2Command {
    type Error = ChunkMessageError;
    fn read_remaining_from(
        header: amf_formats::Version,
        mut reader: R,
    ) -> Result<Self, Self::Error> {
        let transaction_id = amf_formats::Value::read_number(reader.by_ref(), header)?
            .ok_or_else(|| ChunkMessageError::UnexpectedAmfType {
                amf_type: "expect number type".to_owned(),
                backtrace: Backtrace::capture(),
            })?
            .to_u8()
            .expect("transaction id overflow u8");
        if transaction_id != 0 {
            tracing::warn!(
                "play2 transaction_id should be 0, got {} instead",
                transaction_id
            );
        }
        amf_formats::Value::read_null(reader.by_ref(), header)?.ok_or_else(|| {
            ChunkMessageError::UnexpectedAmfType {
                amf_type: "expect null type".to_owned(),
                backtrace: Backtrace::capture(),
            }
        })?;

        // TODO parameters should be a NetStreamPlayOptions object
        Ok(Play2Command {
            _command_name: c2s_command_names::PLAY2.to_string(),
            _transaction_id: transaction_id,
            parameters: amf_formats::Value::read_object(reader.by_ref(), header)
                .unwrap_or(None)
                .unwrap_or_default(),
        })
    }
}

impl<R: io::Read> ReadRemainingFrom<amf_formats::Version, R> for DeleteStreamCommand {
    type Error = ChunkMessageError;
    fn read_remaining_from(
        header: amf_formats::Version,
        mut reader: R,
    ) -> Result<Self, Self::Error> {
        let transaction_id = amf_formats::Value::read_number(reader.by_ref(), header)?
            .ok_or_else(|| ChunkMessageError::UnexpectedAmfType {
                amf_type: "expect number type".to_owned(),
                backtrace: Backtrace::capture(),
            })?
            .to_u8()
            .expect("transaction id overflow u8");
        if transaction_id != 0 {
            tracing::warn!(
                "deleteStream transaction_id should be 0, got {} instead",
                transaction_id
            );
        }
        amf_formats::Value::read_null(reader.by_ref(), header)?.ok_or_else(|| {
            ChunkMessageError::UnexpectedAmfType {
                amf_type: "expect null type".to_owned(),
                backtrace: Backtrace::capture(),
            }
        })?;
        let stream_id = amf_formats::Value::read_number(reader, header)?.ok_or_else(|| {
            ChunkMessageError::UnexpectedAmfType {
                amf_type: "expect number type".to_owned(),
                backtrace: Backtrace::capture(),
            }
        })?;
        Ok(DeleteStreamCommand {
            _command_name: c2s_command_names::DELETE_STREAM.to_string(),
            _transaction_id: transaction_id,
            stream_id,
        })
    }
}

impl<R: io::Read> ReadRemainingFrom<amf_formats::Version, R> for ReceiveAudioCommand {
    type Error = ChunkMessageError;
    fn read_remaining_from(
        header: amf_formats::Version,
        mut reader: R,
    ) -> Result<Self, Self::Error> {
        let transaction_id = amf_formats::Value::read_number(reader.by_ref(), header)?
            .ok_or_else(|| ChunkMessageError::UnexpectedAmfType {
                amf_type: "expect number type".to_owned(),
                backtrace: Backtrace::capture(),
            })?
            .to_u8()
            .expect("transaction id overflow u8");

        if transaction_id != 0 {
            tracing::warn!(
                "receiveAudio transaction_id should be 0, got {} instead",
                transaction_id
            );
        }

        amf_formats::Value::read_null(reader.by_ref(), header)?.ok_or_else(|| {
            ChunkMessageError::UnexpectedAmfType {
                amf_type: "expect null type".to_owned(),
                backtrace: Backtrace::capture(),
            }
        })?;
        let bool_flag = amf_formats::Value::read_bool(reader, header)?.ok_or_else(|| {
            ChunkMessageError::UnexpectedAmfType {
                amf_type: "expect bool type".to_owned(),
                backtrace: Backtrace::capture(),
            }
        })?;
        Ok(ReceiveAudioCommand {
            _command_name: c2s_command_names::RECEIVE_AUDIO.to_string(),
            _transaction_id: transaction_id,
            bool_flag,
        })
    }
}

impl<R: io::Read> ReadRemainingFrom<amf_formats::Version, R> for ReceiveVideoCommand {
    type Error = ChunkMessageError;
    fn read_remaining_from(
        header: amf_formats::Version,
        mut reader: R,
    ) -> Result<Self, Self::Error> {
        let transaction_id = amf_formats::Value::read_number(reader.by_ref(), header)?
            .ok_or_else(|| ChunkMessageError::UnexpectedAmfType {
                amf_type: "expect number type".to_owned(),
                backtrace: Backtrace::capture(),
            })?
            .to_u8()
            .expect("transaction id overflow u8");

        if transaction_id != 0 {
            tracing::warn!(
                "receiveVideo transaction_id should be 0, got {} instead",
                transaction_id
            );
        }

        amf_formats::Value::read_null(reader.by_ref(), header)?.ok_or_else(|| {
            ChunkMessageError::UnexpectedAmfType {
                amf_type: "expect null type".to_owned(),
                backtrace: Backtrace::capture(),
            }
        })?;
        let bool_flag = amf_formats::Value::read_bool(reader, header)?.ok_or_else(|| {
            ChunkMessageError::UnexpectedAmfType {
                amf_type: "expect bool type".to_owned(),
                backtrace: Backtrace::capture(),
            }
        })?;
        Ok(ReceiveVideoCommand {
            _command_name: c2s_command_names::RECEIVE_VIDEO.to_string(),
            _transaction_id: transaction_id,
            bool_flag,
        })
    }
}

impl<R: io::Read> ReadRemainingFrom<amf_formats::Version, R> for PublishCommand {
    type Error = ChunkMessageError;
    fn read_remaining_from(
        header: amf_formats::Version,
        mut reader: R,
    ) -> Result<Self, Self::Error> {
        let transaction_id = amf_formats::Value::read_number(reader.by_ref(), header)?
            .ok_or_else(|| ChunkMessageError::UnexpectedAmfType {
                amf_type: "expect number type".to_owned(),
                backtrace: Backtrace::capture(),
            })?
            .to_u8()
            .expect("transaction id overflow u8");

        if transaction_id != 0 {
            tracing::warn!(
                "publish transaction_id should be 0, got {} instead",
                transaction_id
            );
        }

        amf_formats::Value::read_null(reader.by_ref(), header)?.ok_or_else(|| {
            ChunkMessageError::UnexpectedAmfType {
                amf_type: "expect null type".to_owned(),
                backtrace: Backtrace::capture(),
            }
        })?;
        let publishing_name = amf_formats::Value::read_string(reader.by_ref(), header)?
            .ok_or_else(|| ChunkMessageError::UnexpectedAmfType {
                amf_type: "expect string type".to_owned(),
                backtrace: Backtrace::capture(),
            })?;
        let publishing_type =
            amf_formats::Value::read_string(reader, header)?.ok_or_else(|| {
                ChunkMessageError::UnexpectedAmfType {
                    amf_type: "expect string type".to_owned(),
                    backtrace: Backtrace::capture(),
                }
            })?;
        if publishing_type != "live" && publishing_type != "record" && publishing_type != "append" {
            return Err(ChunkMessageError::UnexpectedAmfType {
                amf_type: format!(
                    "expect publish type to be live, record or append, got {}",
                    publishing_type
                ),
                backtrace: Backtrace::capture(),
            });
        }

        Ok(PublishCommand {
            _command_name: c2s_command_names::PUBLISH.to_string(),
            _transaction_id: transaction_id,
            publishing_name,
            publishing_type,
        })
    }
}

impl<R: io::Read> ReadRemainingFrom<amf_formats::Version, R> for SeekCommand {
    type Error = ChunkMessageError;
    fn read_remaining_from(
        header: amf_formats::Version,
        mut reader: R,
    ) -> Result<Self, Self::Error> {
        let transaction_id = amf_formats::Value::read_number(reader.by_ref(), header)?
            .ok_or_else(|| ChunkMessageError::UnexpectedAmfType {
                amf_type: "expect number type".to_owned(),
                backtrace: Backtrace::capture(),
            })?
            .to_u8()
            .expect("transaction id overflow u8");
        if transaction_id != 0 {
            tracing::warn!(
                "seek transaction_id should be 0, got {} instead",
                transaction_id
            );
        }

        amf_formats::Value::read_null(reader.by_ref(), header)?.ok_or_else(|| {
            ChunkMessageError::UnexpectedAmfType {
                amf_type: "expect null type".to_owned(),
                backtrace: Backtrace::capture(),
            }
        })?;
        let milliseconds = amf_formats::Value::read_number(reader, header)?
            .ok_or_else(|| ChunkMessageError::UnexpectedAmfType {
                amf_type: "expect number type".to_owned(),
                backtrace: Backtrace::capture(),
            })?
            .to_u64()
            .expect("milliseconds overflow u64");
        Ok(SeekCommand {
            _command_name: c2s_command_names::SEEK.to_string(),
            _transaction_id: transaction_id,
            milliseconds,
        })
    }
}

impl<R: io::Read> ReadRemainingFrom<amf_formats::Version, R> for PauseCommand {
    type Error = ChunkMessageError;
    fn read_remaining_from(
        header: amf_formats::Version,
        mut reader: R,
    ) -> Result<Self, Self::Error> {
        let transaction_id = amf_formats::Value::read_number(reader.by_ref(), header)?
            .ok_or_else(|| ChunkMessageError::UnexpectedAmfType {
                amf_type: "expect number type".to_owned(),
                backtrace: Backtrace::capture(),
            })?
            .to_u8()
            .expect("transaction id overflow u8");
        if transaction_id != 0 {
            tracing::warn!(
                "pause transaction_id should be 0, got {} instead",
                transaction_id
            );
        }
        amf_formats::Value::read_null(reader.by_ref(), header)?.ok_or_else(|| {
            ChunkMessageError::UnexpectedAmfType {
                amf_type: "expect null type".to_owned(),
                backtrace: Backtrace::capture(),
            }
        })?;
        let pause_flag =
            amf_formats::Value::read_bool(reader.by_ref(), header)?.ok_or_else(|| {
                ChunkMessageError::UnexpectedAmfType {
                    amf_type: "expect bool type".to_owned(),
                    backtrace: Backtrace::capture(),
                }
            })?;
        let milliseconds = amf_formats::Value::read_number(reader.by_ref(), header)?
            .ok_or_else(|| ChunkMessageError::UnexpectedAmfType {
                amf_type: "expect number type".to_owned(),
                backtrace: Backtrace::capture(),
            })?
            .to_u64()
            .expect("milliseconds overflow u64");
        Ok(PauseCommand {
            _command_name: c2s_command_names::PAUSE.to_string(),
            _transaction_id: transaction_id,
            pause_flag,
            milliseconds,
        })
    }
}

impl<R: io::Read> ReadRemainingFrom<(amf_formats::Version, String), R> for CallCommandRequest {
    type Error = ChunkMessageError;
    fn read_remaining_from(
        header: (amf_formats::Version, String),
        mut reader: R,
    ) -> Result<Self, Self::Error> {
        let transaction_id = amf_formats::Value::read_number(reader.by_ref(), header.0)?
            .ok_or_else(|| ChunkMessageError::UnexpectedAmfType {
                amf_type: "expect number type".to_owned(),
                backtrace: Backtrace::capture(),
            })?;
        let command_object = amf_formats::Value::read_object(reader.by_ref(), header.0)?;

        let optional_arguments = amf_formats::Value::read_remaining_from(header.0, reader).ok();
        Ok(CallCommandRequest {
            procedure_name: header.1,
            transaction_id,
            command_object,
            optional_arguments: optional_arguments.map(Either::Left),
        })
    }
}

impl<R: io::Read> ReadRemainingFrom<(amf_formats::Version, RtmpS2CCommandsType), R>
    for RtmpS2CCommands
{
    type Error = ChunkMessageError;
    fn read_remaining_from(
        header: (amf_formats::Version, RtmpS2CCommandsType),
        reader: R,
    ) -> Result<Self, Self::Error> {
        match header.1 {
            RtmpS2CCommandsType::Connect => Ok(RtmpS2CCommands::Connect(
                ConnectCommandResponse::read_remaining_from(header.0, reader)?,
            )),
            RtmpS2CCommandsType::Call => Ok(RtmpS2CCommands::Call(
                CallCommandResponse::read_remaining_from(header.0, reader)?,
            )),
            RtmpS2CCommandsType::CreateStream => Ok(RtmpS2CCommands::CreateStream(
                CreateStreamCommandResponse::read_remaining_from(header.0, reader)?,
            )),
            RtmpS2CCommandsType::OnStatus => Ok(RtmpS2CCommands::OnStatus(
                OnStatusCommand::read_remaining_from(header.0, reader)?,
            )),
        }
    }
}

impl<R: io::Read> ReadRemainingFrom<amf_formats::Version, R> for ConnectCommandResponse {
    type Error = ChunkMessageError;
    fn read_remaining_from(
        header: amf_formats::Version,
        mut reader: R,
    ) -> Result<Self, Self::Error> {
        let command_name =
            amf_formats::Value::read_string(reader.by_ref(), header)?.ok_or_else(|| {
                ChunkMessageError::UnexpectedAmfType {
                    amf_type: "expect string type".to_owned(),
                    backtrace: Backtrace::capture(),
                }
            })?;
        if command_name != s2c_command_names::RESULT && command_name != s2c_command_names::ERROR {
            return Err(ChunkMessageError::UnexpectedCommandName(format!(
                "expect _result or _error, got: {}",
                command_name
            )));
        }

        let transaction_id = amf_formats::Value::read_number(reader.by_ref(), header)?
            .ok_or_else(|| ChunkMessageError::UnexpectedAmfType {
                amf_type: "expect number type".to_owned(),
                backtrace: Backtrace::capture(),
            })?
            .to_u8()
            .expect("transaction id overflow u8");
        if transaction_id != 1 {
            tracing::warn!(
                "connect response transaction_id should be 1, got {} instead",
                transaction_id
            );
        }

        let properties = amf_formats::Value::read_object(reader.by_ref(), header).unwrap_or(None);
        let information = amf_formats::Value::read_remaining_from(header, reader).ok();

        Ok(ConnectCommandResponse {
            success: command_name == s2c_command_names::RESULT,
            transaction_id,
            properties,
            information: information.map(Either::Left),
        })
    }
}

impl<R: io::Read> ReadRemainingFrom<amf_formats::Version, R> for CallCommandResponse {
    type Error = ChunkMessageError;
    fn read_remaining_from(
        header: amf_formats::Version,
        mut reader: R,
    ) -> Result<Self, Self::Error> {
        let command_name =
            amf_formats::Value::read_string(reader.by_ref(), header)?.ok_or_else(|| {
                ChunkMessageError::UnexpectedAmfType {
                    amf_type: "expect string type".to_owned(),
                    backtrace: Backtrace::capture(),
                }
            })?;
        let transaction_id =
            amf_formats::Value::read_number(reader.by_ref(), header)?.ok_or_else(|| {
                ChunkMessageError::UnexpectedAmfType {
                    amf_type: "expect number type".to_owned(),
                    backtrace: Backtrace::capture(),
                }
            })?;
        let command_object =
            amf_formats::Value::read_object(reader.by_ref(), header).unwrap_or(None);
        let response = amf_formats::Value::read_object(reader, header).unwrap_or(None);
        Ok(CallCommandResponse {
            command_name,
            transaction_id,
            command_object,
            response,
        })
    }
}

impl<R: io::Read> ReadRemainingFrom<amf_formats::Version, R> for CreateStreamCommandResponse {
    type Error = ChunkMessageError;
    fn read_remaining_from(
        header: amf_formats::Version,
        mut reader: R,
    ) -> Result<Self, Self::Error> {
        let command_name =
            amf_formats::Value::read_string(reader.by_ref(), header)?.ok_or_else(|| {
                ChunkMessageError::UnexpectedAmfType {
                    amf_type: "expect string type".to_owned(),
                    backtrace: Backtrace::capture(),
                }
            })?;
        if command_name != s2c_command_names::RESULT && command_name != s2c_command_names::ERROR {
            return Err(ChunkMessageError::UnexpectedCommandName(format!(
                "expect _result or _error, got: {}",
                command_name
            )));
        }

        let transaction_id =
            amf_formats::Value::read_number(reader.by_ref(), header)?.ok_or_else(|| {
                ChunkMessageError::UnexpectedAmfType {
                    amf_type: "expect number type".to_owned(),
                    backtrace: Backtrace::capture(),
                }
            })?;
        let command_object = amf_formats::Value::read_object(reader.by_ref(), header)?;
        let stream_id = amf_formats::Value::read_number(reader, header)?.ok_or_else(|| {
            ChunkMessageError::UnexpectedAmfType {
                amf_type: "expect number type".to_owned(),
                backtrace: Backtrace::capture(),
            }
        })?;
        Ok(CreateStreamCommandResponse {
            // command_name,
            success: command_name == s2c_command_names::RESULT,
            transaction_id,
            command_object,
            stream_id,
        })
    }
}

impl<R: io::Read> ReadRemainingFrom<amf_formats::Version, R> for OnStatusCommand {
    type Error = ChunkMessageError;
    fn read_remaining_from(
        header: amf_formats::Version,
        mut reader: R,
    ) -> Result<Self, Self::Error> {
        let command_name =
            amf_formats::Value::read_string(reader.by_ref(), header)?.ok_or_else(|| {
                ChunkMessageError::UnexpectedAmfType {
                    amf_type: "expect string type".to_owned(),
                    backtrace: Backtrace::capture(),
                }
            })?;
        if command_name != s2c_command_names::ON_STATUS {
            return Err(ChunkMessageError::UnexpectedCommandName(format!(
                "expect onStatus, got: {}",
                command_name
            )));
        }

        let transaction_id = amf_formats::Value::read_number(reader.by_ref(), header)?
            .ok_or_else(|| ChunkMessageError::UnexpectedAmfType {
                amf_type: "expect number type".to_owned(),
                backtrace: Backtrace::capture(),
            })?
            .to_u8()
            .expect("transaction id overflow u8");
        if transaction_id != 0 {
            tracing::warn!(
                "onStatus transaction_id should be 0, got {} instead",
                transaction_id
            );
        }

        amf_formats::Value::read_null(reader.by_ref(), header)?.ok_or_else(|| {
            ChunkMessageError::UnexpectedAmfType {
                amf_type: "expect null type".to_owned(),
                backtrace: Backtrace::capture(),
            }
        })?;

        let info_object = amf_formats::Value::read_object(reader, header)?;
        match &info_object {
            None => {
                return Err(ChunkMessageError::UnexpectedAmfType {
                    amf_type: "expect key-value pair type, got a null".to_string(),
                    backtrace: Backtrace::capture(),
                });
            }
            Some(map) => {
                let level = map.get("level");
                match level {
                    None => {
                        return Err(ChunkMessageError::UnexpectedAmfType {
                            amf_type: "expect a level field".to_string(),
                            backtrace: Backtrace::capture(),
                        });
                    }
                    Some(value) => {
                        let str = value.try_as_str();
                        match str {
                            None => {
                                return Err(ChunkMessageError::UnexpectedAmfType {
                                    amf_type: format!(
                                        "expect level field to be string, got a {:?}",
                                        value
                                    ),
                                    backtrace: Backtrace::capture(),
                                });
                            }
                            Some(str) => {
                                if str != "warning" && str != "status" && str != "error" {
                                    return Err(ChunkMessageError::UnexpectedAmfType {
                                        amf_type: format!(
                                            "expect level field value to be warning, status or error. got: {}",
                                            str
                                        ),
                                        backtrace: Backtrace::capture(),
                                    });
                                }
                            }
                        }
                    }
                };
                if map.get("code").is_none() {
                    return Err(ChunkMessageError::UnexpectedAmfType {
                        amf_type: "expect a code field".to_string(),
                        backtrace: Backtrace::capture(),
                    });
                };
                if map.get("description").is_none() {
                    return Err(ChunkMessageError::UnexpectedAmfType {
                        amf_type: "expect a description field".to_string(),
                        backtrace: Backtrace::capture(),
                    });
                }
            }
        }
        Ok(OnStatusCommand {
            command_name,
            transaction_id,
            info_object: info_object.expect("there must be some value"),
        })
    }
}
