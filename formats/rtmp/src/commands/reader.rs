use crate::{
    chunk::errors::{ChunkMessageError, ChunkMessageResult},
    commands::consts::s2c_command_names,
};

use super::{
    CallCommandRequest, CallCommandResponse, ConnectCommandRequest, ConnectCommandRequestObject,
    ConnectCommandResponse, CreateStreamCommandRequest, CreateStreamCommandResponse,
    DeleteStreamCommand, OnStatusCommand, PauseCommand, Play2Command, PlayCommand, PublishCommand,
    ReceiveAudioCommand, ReceiveVideoCommand, RtmpC2SCommands, RtmpS2CCommands,
    RtmpS2CCommandsType, SeekCommand, consts::c2s_command_names,
};
use amf::{Value as AmfValue, amf0::Value as Amf0Value, amf3::Value as Amf3Value};
use std::{collections::HashMap, io};

#[derive(Debug)]
pub struct Reader<R> {
    inner: R,
    amf_version: amf::Version,
}

impl<R> Reader<R>
where
    R: io::Read,
{
    pub fn new(inner: R, amf_version: amf::Version) -> Self {
        Self { inner, amf_version }
    }

    pub fn read_c2s_command(&mut self) -> ChunkMessageResult<RtmpC2SCommands> {
        let command_name = self.read_amf_string()?;

        match command_name.as_str() {
            c2s_command_names::CONNECT => {
                Ok(RtmpC2SCommands::Connect(self.read_c2s_connect_command()?))
            }
            c2s_command_names::CLOSE => todo!(), // FIXME no spec on this one
            c2s_command_names::CREATE_STREAM => Ok(RtmpC2SCommands::CreateStream(
                self.read_c2s_create_stream_command()?,
            )),
            c2s_command_names::PLAY => Ok(RtmpC2SCommands::Play(self.read_c2s_play_command()?)),
            c2s_command_names::PLAY2 => Ok(RtmpC2SCommands::Play2(self.read_c2s_play2_command()?)),
            c2s_command_names::DELETE_STREAM => Ok(RtmpC2SCommands::DeleteStream(
                self.read_c2s_delete_stream_command()?,
            )),
            c2s_command_names::CLOSE_STREAM => todo!(), // FIXME no spec on this one
            c2s_command_names::RECEIVE_AUDIO => Ok(RtmpC2SCommands::ReceiveAudio(
                self.read_c2s_receive_audio_command()?,
            )),
            c2s_command_names::RECEIVE_VIDEO => Ok(RtmpC2SCommands::ReceiveVideo(
                self.read_c2s_receive_video_command()?,
            )),
            c2s_command_names::PUBLISH => {
                Ok(RtmpC2SCommands::Publish(self.read_c2s_publish_command()?))
            }
            c2s_command_names::SEEK => Ok(RtmpC2SCommands::Seek(self.read_c2s_seek_command()?)),
            c2s_command_names::PAUSE => Ok(RtmpC2SCommands::Pause(self.read_c2s_pause_command()?)),
            procedure_name => Ok(RtmpC2SCommands::Call(
                self.read_c2s_call_command(procedure_name.to_string())?,
            )), // call
        }
    }

    pub fn read_s2c_command(
        &mut self,
        command_type: RtmpS2CCommandsType,
    ) -> ChunkMessageResult<RtmpS2CCommands> {
        match command_type {
            RtmpS2CCommandsType::Connect => {
                Ok(RtmpS2CCommands::Connect(self.read_s2c_connect_command()?))
            }
            RtmpS2CCommandsType::Call => Ok(RtmpS2CCommands::Call(self.read_s2c_call_command()?)),
            RtmpS2CCommandsType::CreateStream => Ok(RtmpS2CCommands::CreateStream(
                self.read_s2c_create_stream_command()?,
            )),
            RtmpS2CCommandsType::OnStatus => Ok(RtmpS2CCommands::OnStatus(
                self.read_s2c_on_status_command()?,
            )),
        }
    }

    fn read_c2s_connect_command(&mut self) -> ChunkMessageResult<ConnectCommandRequest> {
        let transaction_id = self.read_amf_number()? as u8;
        assert!(transaction_id == 1, "connect transaction_id should be 1");
        let command_object_map = self.read_amf_object()?;
        if command_object_map.is_none() {
            return Err(ChunkMessageError::UnexpectedAmfType(
                "expect a key-value pair type".to_string(),
            ));
        }
        let command_object_map = command_object_map.expect("there must be some value");
        let command_object: ConnectCommandRequestObject = command_object_map.try_into()?;

        let optional_user_arguments = self.read_amf_object()?;
        Ok(ConnectCommandRequest {
            command_name: c2s_command_names::CONNECT.to_string(),
            transaction_id,
            command_object,
            optional_user_arguments,
        })
    }

    fn read_s2c_connect_command(&mut self) -> ChunkMessageResult<ConnectCommandResponse> {
        let command_name = self.read_amf_string()?;
        if command_name != s2c_command_names::RESULT && command_name != s2c_command_names::ERROR {
            return Err(ChunkMessageError::UnexpectedCommandName(format!(
                "expect _result or _error, got: {}",
                command_name
            )));
        }

        let transaction_id = self.read_amf_number()? as u8;
        assert!(
            transaction_id == 1,
            "connect response transaction_id should be 1"
        );
        let properties = self.read_amf_object()?;
        let information = self.read_amf_remaining_any()?.unwrap_or(Vec::new());
        let mut information_map = HashMap::new();
        for v in information {
            let pairs = v.try_into_pairs();
            match pairs {
                Ok(pairs) => {
                    for (k, v) in pairs {
                        information_map.insert(k, v);
                    }
                }
                _ => {
                    return Err(ChunkMessageError::UnexpectedAmfType(
                        "expect key-value pairs".to_string(),
                    ));
                }
            }
        }
        Ok(ConnectCommandResponse {
            success: command_name == s2c_command_names::RESULT,
            transaction_id,
            properties,
            information: information_map,
        })
    }

    fn read_c2s_call_command(
        &mut self,
        procedure_name: String,
    ) -> ChunkMessageResult<CallCommandRequest> {
        let transaction_id = self.read_amf_number()?;
        let command_object = self.read_amf_object()?;

        let optional_arguments = self.read_amf_object()?;
        Ok(CallCommandRequest {
            procedure_name,
            transaction_id,
            command_object,
            optional_arguments,
        })
    }

    fn read_s2c_call_command(&mut self) -> ChunkMessageResult<CallCommandResponse> {
        let command_name = self.read_amf_string()?;
        let transaction_id = self.read_amf_number()?;
        let command_object = self.read_amf_object()?;
        let response = self.read_amf_object()?;
        Ok(CallCommandResponse {
            command_name,
            transaction_id,
            command_object,
            response,
        })
    }

    fn read_c2s_create_stream_command(&mut self) -> ChunkMessageResult<CreateStreamCommandRequest> {
        let transaction_id = self.read_amf_number()?;

        let command_object = self.read_amf_object()?;
        Ok(CreateStreamCommandRequest {
            command_name: c2s_command_names::CREATE_STREAM.to_string(),
            transaction_id,
            command_object,
        })
    }

    fn read_s2c_create_stream_command(
        &mut self,
    ) -> ChunkMessageResult<CreateStreamCommandResponse> {
        let command_name = self.read_amf_string()?;
        if command_name != s2c_command_names::RESULT && command_name != s2c_command_names::ERROR {
            return Err(ChunkMessageError::UnexpectedCommandName(format!(
                "expect _result or _error, got: {}",
                command_name
            )));
        }

        let transaction_id = self.read_amf_number()?;
        let command_object = self.read_amf_object()?;
        let stream_id = self.read_amf_number()?;
        Ok(CreateStreamCommandResponse {
            // command_name,
            success: command_name == s2c_command_names::RESULT,
            transaction_id,
            command_object,
            stream_id,
        })
    }

    fn read_s2c_on_status_command(&mut self) -> ChunkMessageResult<OnStatusCommand> {
        let command_name = self.read_amf_string()?;
        if command_name != s2c_command_names::ON_STATUS {
            return Err(ChunkMessageError::UnexpectedCommandName(format!(
                "expect onStatus, got: {}",
                command_name
            )));
        }

        let transaction_id = self.read_amf_number()?;
        assert!(
            transaction_id == 0 as f64,
            "onStatus transaction_id should be 0"
        );
        self.read_amf_null()?;

        let info_object = self.read_amf_object()?;
        match &info_object {
            None => {
                return Err(ChunkMessageError::UnexpectedAmfType(
                    "expect key-value pair type, got a null".to_string(),
                ));
            }
            Some(map) => {
                let level = map.get("level");
                match level {
                    None => {
                        return Err(ChunkMessageError::UnexpectedAmfType(
                            "expect a level field".to_string(),
                        ));
                    }
                    Some(value) => {
                        let str = value.try_as_str();
                        match str {
                            None => {
                                return Err(ChunkMessageError::UnexpectedAmfType(format!(
                                    "expect level field to be string, got a {:?}",
                                    value
                                )));
                            }
                            Some(str) => {
                                if str != "warning" && str != "status" && str != "error" {
                                    return Err(ChunkMessageError::UnexpectedAmfType(format!(
                                        "expect level field value to be warning, status or error. got: {}",
                                        str
                                    )));
                                }
                            }
                        }
                    }
                };
                match map.get("code") {
                    None => {
                        return Err(ChunkMessageError::UnexpectedAmfType(
                            "expect a code field".to_string(),
                        ));
                    }
                    _ => {}
                };
                match map.get("description") {
                    None => {
                        return Err(ChunkMessageError::UnexpectedAmfType(
                            "expect a description field".to_string(),
                        ));
                    }
                    _ => {}
                };
            }
        }
        Ok(OnStatusCommand {
            command_name,
            transaction_id: transaction_id as u8,
            info_object: info_object.expect("there must be some value"),
        })
    }

    fn read_c2s_play_command(&mut self) -> ChunkMessageResult<PlayCommand> {
        let transaction_id = self.read_amf_number()? as u8;
        assert!(transaction_id == 0, "play transaction_id should be 0");
        self.read_amf_null()?;
        let stream_name = self.read_amf_string()?;
        let start = self.read_amf_number()? as i64;
        let duration = self.read_amf_number()? as i64;
        let reset = self.read_amf_bool()?;
        Ok(PlayCommand {
            command_name: c2s_command_names::PLAY.to_string(),
            transaction_id,
            stream_name,
            start,
            duration,
            reset,
        })
    }

    fn read_c2s_play2_command(&mut self) -> ChunkMessageResult<Play2Command> {
        let transaction_id = self.read_amf_number()? as u8;
        assert!(transaction_id == 0, "play2 transaction_id should be 0");
        self.read_amf_null()?;
        // TODO parameters should be a NetStreamPlayOptions object
        Ok(Play2Command {
            command_name: c2s_command_names::PLAY2.to_string(),
            transaction_id,
            parameters: self.read_amf_object()?.unwrap_or(HashMap::default()),
        })
    }

    fn read_c2s_delete_stream_command(&mut self) -> ChunkMessageResult<DeleteStreamCommand> {
        let transaction_id = self.read_amf_number()? as u8;
        assert!(
            transaction_id == 0,
            "deleteStream transaction_id should be 0"
        );
        self.read_amf_null()?;
        let stream_id = self.read_amf_number()?;
        Ok(DeleteStreamCommand {
            command_name: c2s_command_names::DELETE_STREAM.to_string(),
            transaction_id,
            stream_id,
        })
    }

    fn read_c2s_receive_audio_command(&mut self) -> ChunkMessageResult<ReceiveAudioCommand> {
        let transaction_id = self.read_amf_number()? as u8;
        assert!(
            transaction_id == 0,
            "receiveAudio transaction_id should be 0"
        );
        self.read_amf_null()?;
        let bool_flag = self.read_amf_bool()?;
        Ok(ReceiveAudioCommand {
            command_name: c2s_command_names::RECEIVE_AUDIO.to_string(),
            transaction_id,
            bool_flag,
        })
    }

    fn read_c2s_receive_video_command(&mut self) -> ChunkMessageResult<ReceiveVideoCommand> {
        let transaction_id = self.read_amf_number()? as u8;
        assert!(
            transaction_id == 0,
            "receiveVideo transaction_id should be 0"
        );
        self.read_amf_null()?;
        let bool_flag = self.read_amf_bool()?;
        Ok(ReceiveVideoCommand {
            command_name: c2s_command_names::RECEIVE_VIDEO.to_string(),
            transaction_id,
            bool_flag,
        })
    }

    fn read_c2s_publish_command(&mut self) -> ChunkMessageResult<PublishCommand> {
        let transaction_id = self.read_amf_number()? as u8;
        assert!(transaction_id == 0, "publish transaction_id should be 0");
        self.read_amf_null()?;
        let publishing_name = self.read_amf_string()?;
        let publishing_type = self.read_amf_string()?;
        if publishing_type != "live" || publishing_type != "record" || publishing_type != "append" {
            return Err(ChunkMessageError::UnexpectedAmfType(format!(
                "expect publish type to be live, record or append, got {}",
                publishing_type
            )));
        }

        Ok(PublishCommand {
            command_name: c2s_command_names::PUBLISH.to_string(),
            transaction_id,
            publishing_name,
            publishing_type,
        })
    }

    fn read_c2s_seek_command(&mut self) -> ChunkMessageResult<SeekCommand> {
        let transaction_id = self.read_amf_number()? as u8;
        assert!(transaction_id == 0, "seek transaction_id should be 0");
        self.read_amf_null()?;
        let milliseconds = self.read_amf_number()? as u64;
        Ok(SeekCommand {
            command_name: c2s_command_names::SEEK.to_string(),
            transaction_id,
            milliseconds,
        })
    }

    fn read_c2s_pause_command(&mut self) -> ChunkMessageResult<PauseCommand> {
        let transaction_id = self.read_amf_number()? as u8;
        assert!(transaction_id == 0, "pause transaction_id should be 0");
        self.read_amf_null()?;
        let pause_flag = self.read_amf_bool()?;
        let milliseconds = self.read_amf_number()? as u64;
        Ok(PauseCommand {
            command_name: c2s_command_names::PAUSE.to_string(),
            transaction_id,
            pause_flag,
            milliseconds,
        })
    }

    fn read_amf_null(&mut self) -> ChunkMessageResult<()> {
        match AmfValue::read_from(self.inner.by_ref(), self.amf_version)? {
            AmfValue::AMF0Value(Amf0Value::Null) | AmfValue::AMF3Value(Amf3Value::Null) => Ok(()),
            value => Err(ChunkMessageError::UnexpectedAmfType(format!(
                "expect a null type, got a: {:?}",
                value
            ))),
        }
    }

    fn read_amf_string(&mut self) -> ChunkMessageResult<String> {
        match amf::Value::read_from(self.inner.by_ref(), self.amf_version)?.try_as_str() {
            Some(v) => Ok(v.to_string()),
            None => {
                return Err(ChunkMessageError::UnexpectedAmfType(format!(
                    "expect string type",
                )));
            }
        }
    }

    fn read_amf_number(&mut self) -> ChunkMessageResult<f64> {
        match AmfValue::read_from(self.inner.by_ref(), self.amf_version)?.try_as_f64() {
            Some(v) => Ok(v),
            None => {
                return Err(ChunkMessageError::UnexpectedAmfType(format!(
                    "expect a number type"
                )));
            }
        }
    }

    fn read_amf_bool(&mut self) -> ChunkMessageResult<bool> {
        match AmfValue::read_from(self.inner.by_ref(), self.amf_version)?.try_as_bool() {
            Some(v) => Ok(v),
            None => {
                return Err(ChunkMessageError::UnexpectedAmfType(format!(
                    "expect a bool type"
                )));
            }
        }
    }

    fn read_amf_object(&mut self) -> ChunkMessageResult<Option<HashMap<String, AmfValue>>> {
        match AmfValue::read_from(self.inner.by_ref(), self.amf_version)? {
            AmfValue::AMF0Value(Amf0Value::Null) | AmfValue::AMF3Value(Amf3Value::Null) => Ok(None),
            value => match value.try_into_pairs() {
                Err(_) => {
                    return Err(ChunkMessageError::UnexpectedAmfType(
                        "expect key-value pair type".to_string(),
                    ));
                }
                Ok(pairs) => {
                    let mut map = HashMap::new();
                    for (k, v) in pairs {
                        map.insert(k, v);
                    }
                    Ok(Some(map))
                }
            },
        }
    }

    fn read_amf_remaining_any(&mut self) -> ChunkMessageResult<Option<Vec<AmfValue>>> {
        match AmfValue::read_all(self.inner.by_ref(), self.amf_version) {
            Ok(value) if value.len() > 0 => Ok(Some(value)),
            Ok(_) => Ok(None),
            _ => Ok(None),
        }
    }
}
