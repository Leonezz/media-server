use std::{collections::HashMap, io};

use amf::AmfComplexObject;
use tokio_util::either::Either;

use crate::chunk::errors::{ChunkMessageError, ChunkMessageResult};

pub mod consts;
pub mod errors;
pub mod reader;
pub mod writer;

/// The [audio|video]FourCcInfoMap properties are designed to enable setting capability flags
/// for each supported codec in the context of E-RTMP streaming.
/// A FourCC key is a four-character code used to specify a video or audio codec.
/// The names of the object properties are strings that correspond to these FourCC keys.
/// Each object property holds a numeric value that represents a set of capability flags.
/// These flags can be combined using a Bitwise OR operation.
/// Capability flags define specific functionalities, such as the ability to decode, encode, or forward.
/// A FourCC key set to the wildcard character "*" acts as a catch-all for any codec.
/// When this wildcard key exists, it overrides the flags set on properties for specific codecs.
/// For example, if the flag for the "*" property is set to FourCcInfoMask.CanForward,
/// all codecs will be forwarded regardless of individual flags set on their specific properties.
pub mod four_cc_info_mask {
    pub const CAN_DECODE: u8 = 0x01;
    pub const CAN_ENCODE: u8 = 0x02;
    pub const CAN_FORWARD: u8 = 0x04;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct FourCCInfo {
    pub can_decode: bool,
    pub can_encode: bool,
    pub can_forward: bool,
}

impl From<u8> for FourCCInfo {
    fn from(value: u8) -> Self {
        Self {
            can_decode: (value & four_cc_info_mask::CAN_DECODE) == four_cc_info_mask::CAN_DECODE,
            can_encode: (value & four_cc_info_mask::CAN_ENCODE) == four_cc_info_mask::CAN_ENCODE,
            can_forward: (value & four_cc_info_mask::CAN_FORWARD) == four_cc_info_mask::CAN_FORWARD,
        }
    }
}

impl From<FourCCInfo> for u8 {
    fn from(value: FourCCInfo) -> Self {
        (if value.can_decode {
            four_cc_info_mask::CAN_DECODE
        } else {
            0
        } | if value.can_encode {
            four_cc_info_mask::CAN_ENCODE
        } else {
            0
        } | if value.can_forward {
            four_cc_info_mask::CAN_FORWARD
        } else {
            0
        })
    }
}

///
/// The value represents capability flags which can be combined via a Bitwise OR to indicate
/// which extended set of capabilities (i.e., beyond the legacy [RTMP] specification) are supported via E-RTMP.
/// See enum CapsExMask for the enumerated values representing the assigned bits.
/// If the extended capabilities are expressed elsewhere they will not appear here
/// (e.g., FourCC, HDR or VideoPacketType.Metadata support is not expressed in this property).
/// When a specific flag is encountered:
/// - The implementation might fully handle the feature by applying the appropriate logic.
/// - Alternatively, if full support is not available, the implementation can still parse the bitstream correctly,
///     ensuring graceful degradation.
///   This allows continued operation, even with reduced functionality.
pub mod caps_ex_mask {
    pub const RECONNECT: u8 = 0x01; // Support for reconnection
    pub const MULTI_TRACK: u8 = 0x02; // Support for multitrack
    pub const MOD_EX: u8 = 0x04; // Can parse ModEx signal
    pub const TIMESTAMP_NANO_OFFSET: u8 = 0x08; // Support for nano offset
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CapsExInfo {
    pub support_reconnect: bool,
    pub support_mod_ex: bool,
    pub support_multi_track: bool,
    pub support_timestamp_nano: bool,
}

impl From<u8> for CapsExInfo {
    fn from(value: u8) -> Self {
        Self {
            support_reconnect: (value & caps_ex_mask::RECONNECT) == caps_ex_mask::RECONNECT,
            support_mod_ex: (value & caps_ex_mask::MOD_EX) == caps_ex_mask::MOD_EX,
            support_multi_track: (value & caps_ex_mask::MULTI_TRACK) == caps_ex_mask::MULTI_TRACK,
            support_timestamp_nano: (value & caps_ex_mask::TIMESTAMP_NANO_OFFSET)
                == caps_ex_mask::TIMESTAMP_NANO_OFFSET,
        }
    }
}

impl From<CapsExInfo> for u8 {
    fn from(value: CapsExInfo) -> Self {
        (if value.support_reconnect {
            caps_ex_mask::RECONNECT
        } else {
            0
        } | if value.support_mod_ex {
            caps_ex_mask::MOD_EX
        } else {
            0
        } | if value.support_multi_track {
            caps_ex_mask::MULTI_TRACK
        } else {
            0
        } | if value.support_timestamp_nano {
            caps_ex_mask::TIMESTAMP_NANO_OFFSET
        } else {
            0
        })
    }
}

// @see: 7.2.1.1. connect
#[derive(Debug, Clone, Default)]
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
    // the below are from enhanced rtmp
    pub caps_ex_info: Option<CapsExInfo>,
    pub four_cc_list: Option<Vec<String>>,
    pub video_four_cc_info: Option<HashMap<String, FourCCInfo>>,
    pub audio_four_cc_info: Option<HashMap<String, FourCCInfo>>,
}

impl TryFrom<HashMap<String, amf::Value>> for ConnectCommandRequestObject {
    type Error = ChunkMessageError;
    fn try_from(value: HashMap<String, amf::Value>) -> Result<Self, Self::Error> {
        let extract_string_array_field = |key: &str| match value.extract_array_field(key) {
            Some(values) => {
                let mut result = vec![];
                for v in values {
                    if let Some(str_value) = v.try_as_str() {
                        result.push(str_value.to_string());
                    } else {
                        return None;
                    }
                }
                Some(result)
            }
            None => None,
        };

        let extract_four_cc_info = |key: &str| {
            value.extract_object_field(key).map(|pairs| {
                let mut info: HashMap<String, FourCCInfo> = HashMap::new();
                for (k, v) in pairs {
                    if let Some(flag) = v.try_as_f64() {
                        info.insert(k, (flag as u8).into());
                    }
                }
                info
            })
        };

        let command_object = ConnectCommandRequestObject {
            app: value
                .extract_string_field("app")
                .unwrap_or("default".into()),
            flash_version: value
                .extract_string_field("flashver")
                .unwrap_or("default".into()),
            swf_url: value
                .extract_string_field("swfUrl")
                .unwrap_or("default".into()),
            tc_url: value
                .extract_string_field("tcUrl")
                .unwrap_or("default".into()),
            fpad: value.extract_bool_field("fpad").unwrap_or(false),
            audio_codecs: value
                .extract_number_field("audioCodecs")
                .unwrap_or(0.into()) as u16,
            video_codecs: value
                .extract_number_field("videoCodecs")
                .unwrap_or(0.into()) as u16,
            video_function: value
                .extract_number_field("videoFunction")
                .unwrap_or(0.into()) as u16,
            page_url: value
                .extract_string_field("pageUrl")
                .unwrap_or("default".into()),
            object_encoding: match value
                .extract_number_field("objectEncoding")
                .unwrap_or((amf::Version::Amf0 as u8).into())
                as u8
            {
                0 => amf::Version::Amf0,
                3 => amf::Version::Amf3,
                v => return Err(ChunkMessageError::UnknownAmfVersion(v)),
            },
            four_cc_list: extract_string_array_field("fourCcList"),
            video_four_cc_info: extract_four_cc_info("videoFourCcInfoMap"),
            audio_four_cc_info: extract_four_cc_info("audioFourCcInfoMap"),
            caps_ex_info: value
                .extract_number_field("capsEx")
                .map(|v| (v as u8).into()),
        };

        Ok(command_object)
    }
}

impl From<ConnectCommandRequestObject> for HashMap<String, amf::Value> {
    fn from(value: ConnectCommandRequestObject) -> Self {
        let mut map: HashMap<String, amf::Value> = HashMap::new();
        let version = value.object_encoding;
        map.insert("app".into(), amf::string(value.app, version));
        map.insert("flashver".into(), amf::string(value.flash_version, version));
        map.insert("swfUrl".into(), amf::string(value.swf_url, version));
        map.insert("tcUrl".into(), amf::string(value.tc_url, version));
        map.insert("fpad".into(), amf::bool(value.fpad, version));
        map.insert(
            "audioCodecs".into(),
            amf::number(value.audio_codecs, version),
        );
        map.insert(
            "videoCodecs".into(),
            amf::number(value.video_codecs, version),
        );
        map.insert(
            "videoFunction".into(),
            amf::number(value.video_function, version),
        );
        map.insert("pageUrl".into(), amf::string(value.page_url, version));
        map.insert(
            "objectEncoding".into(),
            amf::number::<u8>(
                match value.object_encoding {
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
    _command_name: String, // "play"
    _transaction_id: u8,   // 0
    // command_object is null
    pub stream_name: String,
    pub start: i64,    // default to -2
    pub duration: i64, // default to -1
    pub reset: bool,
}

#[derive(Debug)]
pub struct Play2Command {
    _command_name: String, // "play2"
    _transaction_id: u8,   // 0
    // command_object is null
    parameters: HashMap<String, amf::Value>,
}

#[derive(Debug)]
pub struct DeleteStreamCommand {
    _command_name: String, // "deleteStream"
    _transaction_id: u8,   // 0
    // command_object is null
    stream_id: f64,
}

#[derive(Debug)]
pub struct ReceiveAudioCommand {
    _command_name: String, // "receiveAudio"
    _transaction_id: u8,   // 0
    // command_object is null
    pub bool_flag: bool,
}

#[derive(Debug)]
pub struct ReceiveVideoCommand {
    _command_name: String, // "receiveVideo"
    _transaction_id: u8,   // 0
    // command_object is null
    pub bool_flag: bool,
}

#[derive(Debug)]
pub struct PublishCommand {
    _command_name: String, // "publish"
    _transaction_id: u8,   // 0
    // command_object is null
    pub publishing_name: String, // stream name
    pub publishing_type: String, // "live", "record", "append"
}

#[derive(Debug)]
pub struct SeekCommand {
    _command_name: String, // "seek"
    _transaction_id: u8,   // 0
    // command_object is null
    milliseconds: u64,
}

#[derive(Debug)]
pub struct PauseCommand {
    _command_name: String, // "pause"
    _transaction_id: u8,   // 0
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
        writer::Writer::new(inner, version).write_c2s_command(self)
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
