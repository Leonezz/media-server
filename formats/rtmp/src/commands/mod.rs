use std::collections::HashMap;

use amf::{amf0, amf3};
use errors::CommandMessageError;

pub mod consts;
pub mod errors;
pub mod reader;
pub mod writer;

///! @see: 7.2.1.1. connect
#[derive(Debug)]
pub struct ConnectCommandRequestObject {
    app: String,
    flash_version: String,
    swf_url: String,
    tc_url: String,
    fpad: bool,
    audio_codecs: u16,
    video_codecs: u16,
    video_function: u16,
    page_url: String,
    object_encoding: amf::Version,
}

impl TryFrom<HashMap<String, amf::Value>> for ConnectCommandRequestObject {
    type Error = CommandMessageError;
    fn try_from(value: HashMap<String, amf::Value>) -> Result<Self, Self::Error> {
        let extract_string_field = |key: &str| match value.get(key) {
            Some(value) => match value.try_as_str() {
                Some(v) => Ok(v.to_string()),
                None => {
                    return Err(CommandMessageError::UnexpectedAmfType(format!(
                        "expect a string type"
                    )));
                }
            },
            None => {
                return Err(CommandMessageError::UnexpectedAmfType(format!(
                    "expect {} field",
                    key
                )));
            }
        };
        let extract_bool_field = |key: &str| match value.get(key) {
            Some(value) => match value.try_as_bool() {
                Some(v) => Ok(v),
                None => {
                    return Err(CommandMessageError::UnexpectedAmfType(format!(
                        "expect a bool type"
                    )));
                }
            },
            None => {
                return Err(CommandMessageError::UnexpectedAmfType(format!(
                    "expect {} field",
                    key
                )));
            }
        };

        let extract_number_field = |key: &str| match value.get(key) {
            Some(value) => match value.try_as_f64() {
                Some(v) => Ok(v),
                None => {
                    return Err(CommandMessageError::UnexpectedAmfType(format!(
                        "expect a number type"
                    )));
                }
            },
            None => {
                return Err(CommandMessageError::UnexpectedAmfType(format!(
                    "expect {} field",
                    key
                )));
            }
        };

        let command_object = ConnectCommandRequestObject {
            app: extract_string_field("app")?,
            flash_version: extract_string_field("flashver")?,
            swf_url: extract_string_field("swfUrl")?,
            tc_url: extract_string_field("tcUrl")?,
            fpad: extract_bool_field("fpad")?,
            audio_codecs: extract_number_field("audioCodecs")? as u16,
            video_codecs: extract_number_field("videoCodecs")? as u16,
            video_function: extract_number_field("videoFunction")? as u16,
            page_url: extract_string_field("pageUrl")?,
            object_encoding: match extract_number_field("objectEncoding")? as u8 {
                0 => amf::Version::Amf0,
                3 => amf::Version::Amf3,
                v => return Err(CommandMessageError::UnknownAmfVersion(v as u8)),
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
    command_name: String, // "connect"
    transaction_id: u8,   // always 1
    command_object: ConnectCommandRequestObject,
    optional_user_arguments: Option<HashMap<String, amf::Value>>,
}

#[derive(Debug)]
pub struct ConnectCommandResponse {
    // command_name: String, // "_result" or "_error"
    success: bool,
    transaction_id: u8, // always 1
    properties: Option<HashMap<String, amf::Value>>,
    information: HashMap<String, amf::Value>,
}

#[derive(Debug)]
pub struct CallCommandRequest {
    procedure_name: String,
    transaction_id: f64,
    command_object: Option<HashMap<String, amf::Value>>,
    optional_arguments: Option<HashMap<String, amf::Value>>,
}

#[derive(Debug)]
pub struct CallCommandResponse {
    command_name: String,
    transaction_id: f64,
    command_object: Option<HashMap<String, amf::Value>>,
    response: Option<HashMap<String, amf::Value>>,
}

#[derive(Debug)]
pub struct CreateStreamCommandRequest {
    command_name: String, // "createStream"
    transaction_id: f64,
    command_object: Option<HashMap<String, amf::Value>>,
}

#[derive(Debug)]
pub struct CreateStreamCommandResponse {
    // command_name: String, // "_result" or "_error"
    success: bool,
    transaction_id: f64,
    command_object: Option<HashMap<String, amf::Value>>,
    stream_id: f64,
}

#[derive(Debug)]
pub struct OnStatusCommand {
    command_name: String, // "onStatus"
    transaction_id: u8,   // 0
    // command_object is null
    info_object: HashMap<String, amf::Value>, // at least: level, code, description
}

#[derive(Debug)]
pub struct PlayCommand {
    command_name: String, // "play"
    transaction_id: u8,   // 0
    // command_object is null
    stream_name: String,
    start: i64,    // default to -2
    duration: i64, // default to -1
    reset: bool,
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
    bool_flag: bool,
}

#[derive(Debug)]
pub struct ReceiveVideoCommand {
    command_name: String, // "receiveVideo"
    transaction_id: u8,   // 0
    // command_object is null
    bool_flag: bool,
}

#[derive(Debug)]
pub struct PublishCommand {
    command_name: String, // "publish"
    transaction_id: u8,   // 0
    // command_object is null
    publishing_name: String, // stream name
    publishing_type: String, // "live", "record", "append"
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
