use std::{backtrace::Backtrace, collections::HashMap, io};

use tokio_util::either::Either;

use crate::chunk::errors::{ChunkMessageError, ChunkMessageResult};

pub mod consts;
pub mod errors;
pub mod reader;
pub mod writer;

///! @see: 7.2.1.1. connect
#[derive(Debug, Clone)]
pub struct ConnectCommandRequestObject {
    pub app: String,
    pub flash_version: String,
    pub swf_url: String,
    pub tc_url: String,
    pub fpad: bool,
    pub audio_codecs: u16,
    pub video_codecs: u16,
    pub video_function: u16,
    pub page_url: String,
    pub object_encoding: amf::Version,
}

impl TryFrom<HashMap<String, amf::Value>> for ConnectCommandRequestObject {
    type Error = ChunkMessageError;
    fn try_from(value: HashMap<String, amf::Value>) -> Result<Self, Self::Error> {
        let extract_string_field = |key: &str| match value.get(key) {
            Some(value) => match value.try_as_str() {
                Some(v) => Ok(v.to_string()),
                None => {
                    return Err(ChunkMessageError::UnexpectedAmfType {
                        amf_type: format!("expect a string type"),
                        backtrace: Backtrace::capture(),
                    });
                }
            },
            None => {
                return Err(ChunkMessageError::UnexpectedAmfType {
                    amf_type: format!("expect {} field", key),
                    backtrace: Backtrace::capture(),
                });
            }
        };
        let extract_bool_field = |key: &str| match value.get(key) {
            Some(value) => match value.try_as_bool() {
                Some(v) => Ok(v),
                None => {
                    return Err(ChunkMessageError::UnexpectedAmfType {
                        amf_type: format!("expect a bool type"),
                        backtrace: Backtrace::capture(),
                    });
                }
            },
            None => {
                return Err(ChunkMessageError::UnexpectedAmfType {
                    amf_type: format!("expect {} field", key),
                    backtrace: Backtrace::capture(),
                });
            }
        };

        let extract_number_field = |key: &str| match value.get(key) {
            Some(value) => match value.try_as_f64() {
                Some(v) => Ok(v),
                None => {
                    return Err(ChunkMessageError::UnexpectedAmfType {
                        amf_type: format!("expect a number type"),
                        backtrace: Backtrace::capture(),
                    });
                }
            },
            None => {
                return Err(ChunkMessageError::UnexpectedAmfType {
                    amf_type: format!("expect {} field", key),
                    backtrace: Backtrace::capture(),
                });
            }
        };

        let command_object = ConnectCommandRequestObject {
            app: extract_string_field("app").unwrap_or("default".into()),
            flash_version: extract_string_field("flashver").unwrap_or("default".into()),
            swf_url: extract_string_field("swfUrl").unwrap_or("default".into()),
            tc_url: extract_string_field("tcUrl").unwrap_or("default".into()),
            fpad: extract_bool_field("fpad").unwrap_or(false),
            audio_codecs: extract_number_field("audioCodecs").unwrap_or(0.into()) as u16,
            video_codecs: extract_number_field("videoCodecs").unwrap_or(0.into()) as u16,
            video_function: extract_number_field("videoFunction").unwrap_or(0.into()) as u16,
            page_url: extract_string_field("pageUrl").unwrap_or("default".into()),
            object_encoding: match extract_number_field("objectEncoding")
                .unwrap_or((amf::Version::Amf0 as u8).into())
                as u8
            {
                0 => amf::Version::Amf0,
                3 => amf::Version::Amf3,
                v => return Err(ChunkMessageError::UnknownAmfVersion(v as u8)),
            },
        };

        Ok(command_object)
    }
}

impl Into<HashMap<String, amf::Value>> for ConnectCommandRequestObject {
    fn into(self) -> HashMap<String, amf::Value> {
        let mut map: HashMap<String, amf::Value> = HashMap::new();
        let version = self.object_encoding;
        map.insert("app".into(), amf::string(self.app, version));
        map.insert("flashver".into(), amf::string(self.flash_version, version));
        map.insert("swfUrl".into(), amf::string(self.swf_url, version));
        map.insert("tcUrl".into(), amf::string(self.tc_url, version));
        map.insert("fpad".into(), amf::bool(self.fpad, version));
        map.insert(
            "audioCodecs".into(),
            amf::number(self.audio_codecs, version),
        );
        map.insert(
            "videoCodecs".into(),
            amf::number(self.video_codecs, version),
        );
        map.insert(
            "videoFunction".into(),
            amf::number(self.video_function, version),
        );
        map.insert("pageUrl".into(), amf::string(self.page_url, version));
        map.insert(
            "objectEncoding".into(),
            amf::number::<u8>(
                match self.object_encoding {
                    amf::Version::Amf0 => 0,
                    amf::Version::Amf3 => 3,
                },
                version,
            ),
        );
        map
    }
}

#[derive(Debug)]
pub struct ConnectCommandRequest {
    pub command_name: String, // "connect"
    pub transaction_id: u8,   // always 1
    pub command_object: ConnectCommandRequestObject,
    pub optional_user_arguments: Option<HashMap<String, amf::Value>>,
}

#[derive(Debug)]
pub struct ConnectCommandResponse {
    // command_name: String, // "_result" or "_error"
    pub success: bool,
    pub transaction_id: u8, // always 1
    pub properties: Option<HashMap<String, amf::Value>>,
    pub information: Option<Either<amf::Value, HashMap<String, amf::Value>>>,
}

#[derive(Debug)]
pub struct CallCommandRequest {
    pub procedure_name: String,
    pub transaction_id: f64,
    pub command_object: Option<HashMap<String, amf::Value>>,
    pub optional_arguments: Option<Either<amf::Value, HashMap<String, amf::Value>>>,
}

#[derive(Debug)]
pub struct CallCommandResponse {
    pub command_name: String,
    pub transaction_id: f64,
    pub command_object: Option<HashMap<String, amf::Value>>,
    pub response: Option<HashMap<String, amf::Value>>,
}

#[derive(Debug)]
pub struct CreateStreamCommandRequest {
    pub command_name: String, // "createStream"
    pub transaction_id: f64,
    pub command_object: Option<HashMap<String, amf::Value>>,
}

#[derive(Debug)]
pub struct CreateStreamCommandResponse {
    // command_name: String, // "_result" or "_error"
    pub success: bool,
    pub transaction_id: f64,
    pub command_object: Option<HashMap<String, amf::Value>>,
    pub stream_id: f64,
}

#[derive(Debug)]
pub struct OnStatusCommand {
    pub command_name: String, // "onStatus"
    pub transaction_id: u8,   // 0
    // command_object is null
    pub info_object: HashMap<String, amf::Value>, // at least: level, code, description
}

#[derive(Debug)]
pub struct PlayCommand {
    command_name: String, // "play"
    transaction_id: u8,   // 0
    // command_object is null
    pub stream_name: String,
    pub start: i64,    // default to -2
    pub duration: i64, // default to -1
    pub reset: bool,
}

#[derive(Debug)]
pub struct Play2Command {
    command_name: String, // "play2"
    transaction_id: u8,   // 0
    // command_object is null
    parameters: HashMap<String, amf::Value>,
}

#[derive(Debug)]
pub struct DeleteStreamCommand {
    command_name: String, // "deleteStream"
    transaction_id: u8,   // 0
    // command_object is null
    stream_id: f64,
}

#[derive(Debug)]
pub struct ReceiveAudioCommand {
    command_name: String, // "receiveAudio"
    transaction_id: u8,   // 0
    // command_object is null
    pub bool_flag: bool,
}

#[derive(Debug)]
pub struct ReceiveVideoCommand {
    command_name: String, // "receiveVideo"
    transaction_id: u8,   // 0
    // command_object is null
    pub bool_flag: bool,
}

#[derive(Debug)]
pub struct PublishCommand {
    command_name: String, // "publish"
    transaction_id: u8,   // 0
    // command_object is null
    pub publishing_name: String, // stream name
    pub publishing_type: String, // "live", "record", "append"
}

#[derive(Debug)]
pub struct SeekCommand {
    command_name: String, // "seek"
    transaction_id: u8,   // 0
    // command_object is null
    milliseconds: u64,
}

#[derive(Debug)]
pub struct PauseCommand {
    command_name: String, // "pause"
    transaction_id: u8,   // 0
    // command_object is null
    pause_flag: bool, // pause or unpause
    milliseconds: u64,
}

#[derive(Debug)]
pub enum RtmpC2SCommands {
    Connect(ConnectCommandRequest),
    Call(CallCommandRequest),
    CreateStream(CreateStreamCommandRequest),
    Play(PlayCommand),
    Play2(Play2Command),
    DeleteStream(DeleteStreamCommand),
    ReceiveAudio(ReceiveAudioCommand),
    ReceiveVideo(ReceiveVideoCommand),
    Publish(PublishCommand),
    Seek(SeekCommand),
    Pause(PauseCommand),
}

#[derive(Debug)]
pub enum RtmpS2CCommands {
    Connect(ConnectCommandResponse),
    Call(CallCommandResponse),
    CreateStream(CreateStreamCommandResponse),
    OnStatus(OnStatusCommand),
}

#[derive(Debug)]
pub enum RtmpS2CCommandsType {
    Connect,
    Call,
    CreateStream,
    OnStatus,
}

impl RtmpC2SCommands {
    pub fn read_from<R>(
        inner: R,
        version: amf::Version,
    ) -> Result<RtmpC2SCommands, ChunkMessageError>
    where
        R: io::Read,
    {
        reader::Reader::new(inner, version).read_c2s_command()
    }

    pub fn write_to<W>(&self, inner: W, version: amf::Version) -> ChunkMessageResult<()>
    where
        W: io::Write,
    {
        writer::Writer::new(inner, version).write_c2s_command(&self)
    }
}

impl RtmpS2CCommands {
    pub fn read_from<R>(
        inner: R,
        command_type: RtmpS2CCommandsType,
        version: amf::Version,
    ) -> Result<RtmpS2CCommands, ChunkMessageError>
    where
        R: io::Read,
    {
        reader::Reader::new(inner, version).read_s2c_command(command_type)
    }

    pub fn write_to<W>(&self, inner: W, version: amf::Version) -> ChunkMessageResult<()>
    where
        W: io::Write,
    {
        writer::Writer::new(inner, version).write_s2c_command(self)
    }
}
