use std::{
    io::{BufRead, Cursor, Read, Seek},
    net::{Ipv4Addr, Ipv6Addr},
};

use tokio_util::bytes::Buf;
use url::Url;

use crate::{
    CRLF, LF,
    attributes::SDPAttribute,
    errors::{SDPError, SDPResult},
    session::{
        SDPBandWidthInformation, SDPConnectionInformation, SDPEncryptionKeys, SDPMediaDescription,
        SDPRepeatTime, SDPTimeInformation, SDPTimeZoneAdjustment, SessionDescription,
    },
};

// @see: 9. SDP Grammar
/// ; SDP Syntax
/// session-description = version-field
///                       origin-field
///                       session-name-field
///                       [information-field]
///                       [uri-field]
///                       *email-field
///                       *phone-field
///                       [connection-field]
///                       *bandwidth-field
///                       1*time-description
///                       [key-field]
///                       *attribute-field
///                       *media-description
/// version-field = %s"v" "=" 1*DIGIT CRLF
///                 ;this memo describes version 0
/// origin-field = %s"o" "=" username SP sess-id SP sess-version SP nettype SP addrtype SP unicast-address CRLF
/// session-name-field = %s"s" "=" text CRLF
/// information-field = %s"i" "=" text CRLF
/// uri-field = %s"u" "=" uri CRLF
/// email-field = %s"e" "=" email-address CRLF
/// phone-field = %s"p" "=" phone-number CRLF
/// connection-field = %s"c" "=" nettype SP addrtype SP connection-address CRLF
///                    ;a connection field must be present
///                    ;in every media description or at the
///                    ;session level
/// bandwidth-field = %s"b" "=" bwtype ":" bandwidth CRLF
/// time-description = time-field
///                    [repeat-description]
/// repeat-description = 1*repeat-field
///                      [zone-field]
/// time-field = %s"t" "=" start-time SP stop-time CRLF
/// repeat-field = %s"r" "=" repeat-interval SP typed-time 1*(SP typed-time) CRLF
/// zone-field = %s"z" "=" time SP ["-"] typed-time *(SP time SP ["-"] typed-time) CRLF
/// key-field = %s"k" "=" key-type CRLF
/// attribute-field = %s"a" "=" attribute CRLF
///
/// media-description = media-field
///                     [information-field]
///                     *connection-field
///                     *bandwidth-field
///                     [key-field]
///                     *attribute-field
/// media-field = %s"m" "=" media SP port ["/" integer] SP proto 1*(SP fmt) CRLF
///
/// ; sub-rules of 'o='
/// username = non-ws-string
///            ;pretty wide definition, but doesn't
///            ;include space
/// sess-id = 1*DIGIT
///           ;should be unique for this username/host
/// sess-version = 1*DIGIT
/// nettype = token
///           ;typically "IN"
/// addrtype = token
///            ;typically "IP4" or "IP6"
///
/// ; sub-rules of 'u='
/// uri = URI-reference
///       ; see RFC 3986
///
/// ; sub-rules of 'e=', see RFC 5322 for definitions
/// email-address = address-and-comment / dispname-and-address / addr-spec
/// address-and-comment = addr-spec 1*SP "(" 1*email-safe ")"
/// dispname-and-address = 1*email-safe 1*SP "<" addr-spec ">"
///
/// ; sub-rules of 'p='
/// phone-number = phone *SP "(" 1*email-safe ")" / 1*email-safe "<" phone ">" / phone
/// phone = ["+"] DIGIT 1*(SP / "-" / DIGIT)
///
/// ; sub-rules of 'c='
/// connection-address = multicast-address / unicast-address
///
/// ; sub-rules of 'b='
/// bwtype = token
/// bandwidth = 1*DIGIT
///
/// ; sub-rules of 't='
/// start-time = time / "0"
/// stop-time = time / "0"
/// time = POS-DIGIT 9*DIGIT
///        ; Decimal representation of time in
///        ; seconds since January 1, 1900 UTC.
///        ; The representation is an unbounded
///        ; length field containing at least
///        ; 10 digits. Unlike some representations
///        ; used elsewhere, time in SDP does not
///        ; wrap in the year 2036.
///
/// ; sub-rules of 'r=' and 'z='
/// repeat-interval = POS-DIGIT *DIGIT [fixed-len-time-unit]
/// typed-time = 1*DIGIT [fixed-len-time-unit]
/// fixed-len-time-unit = %s"d" / %s"h" / %s"m" / %s"s"
/// ; NOTE: These units are case-sensitive.
///
/// ; sub-rules of 'k='
/// key-type = %s"prompt" / %s"clear:" text / %s"base64:" base64 / %s"uri:" uri
///            ; NOTE: These names are case-sensitive.
/// base64 = *base64-unit [base64-pad]
/// base64-unit = 4base64-char
/// base64-pad = 2base64-char "==" / 3base64-char "="
/// base64-char = ALPHA / DIGIT / "+" / "/"
///
/// ; sub-rules of 'a='
/// attribute = (attribute-name ":" attribute-value) / attribute-name
/// attribute-name = token
/// attribute-value = byte-string
/// att-field = attribute-name ; for backward compatibility
///
/// ; sub-rules of 'm='
/// media = token
///         ;typically "audio", "video", "text", "image"
///         ;or "application"
/// fmt = token
///       ;typically an RTP payload type for audio
///       ;and video media
/// proto = token *("/" token)
///         ;typically "RTP/AVP", "RTP/SAVP", "udp",
///         ;or "RTP/SAVPF"
/// port = 1*DIGIT
///
/// ; generic sub-rules:
/// addressing unicast-address = IP4-address / IP6-address / FQDN / extn-addr
/// multicast-address = IP4-multicast / IP6-multicast / FQDN / extn-addr
/// IP4-multicast = m1 3( "." decimal-uchar ) "/" ttl [ "/" numaddr ]
///                 ; IP4 multicast addresses may be in the
///                 ; range 224.0.0.0 to 239.255.255.255
/// m1 = ("22" ("4"/"5"/"6"/"7"/"8"/"9")) / ("23" DIGIT )
/// IP6-multicast = IP6-address [ "/" numaddr ]
///                 ; IP6 address starting with FF
/// numaddr = integer
/// ttl = (POS-DIGIT *2DIGIT) / "0"
/// FQDN = 4*(alpha-numeric / "-" / ".")
///        ; fully qualified domain name as specified
///        ; in RFC 1035 (and updates)
/// IP4-address = b1 3("." decimal-uchar)
/// b1 = decimal-uchar
///      ; less than "224"
/// IP6-address =                              6( h16 ":" ) ls32
///               /                       "::" 5( h16 ":" ) ls32
///               /               [ h16 ] "::" 4( h16 ":" ) ls32
///               / [ *1( h16 ":" ) h16 ] "::" 3( h16 ":" ) ls32
///               / [ *2( h16 ":" ) h16 ] "::" 2( h16 ":" ) ls32
///               / [ *3( h16 ":" ) h16 ] "::"    h16   ":" ls32
///               / [ *4( h16 ":" ) h16 ] "::"              ls32
///               / [ *5( h16 ":" ) h16 ] "::"              h16
///               / [ *6( h16 ":" ) h16 ] "::"
/// h16 = 1*4HEXDIG
/// ls32 = ( h16 ":" h16 ) / IP4-address
///
/// ; Generic for other address families
/// extn-addr = non-ws-string
///
/// ; generic sub-rules: datatypes
/// text = byte-string
///        ;default is to interpret this as UTF8 text.
///        ;ISO 8859-1 requires "a=charset:ISO-8859-1"
///        ;session-level attribute to be used
/// byte-string = 1*(%x01-09/%x0B-0C/%x0E-FF)
///               ;any byte except NUL, CR, or LF
/// non-ws-string = 1*(VCHAR/%x80-FF)
///                 ;string of visible characters
/// token-char = ALPHA / DIGIT
///                    / "!" / "#" / "$" / "%" / "&"
///                    / "'" ; (single quote)
///                    / "*" / "+" / "-" / "." / "^" / "_"
///                    / "`" ; (Grave accent)
///                    / "{" / "|" / "}" / "~"
/// token = 1*(token-char)
/// email-safe = %x01-09/%x0B-0C/%x0E-27/%x2A-3B/%x3D/%x3F-FF
///              ;any byte except NUL, CR, LF, or the quoting
///              ;characters ()<>
/// integer = POS-DIGIT *DIGIT
/// zero-based-integer = "0" / integer
/// non-zero-int-or-real = integer / non-zero-real
/// non-zero-real = zero-based-integer "." *DIGIT POS-DIGIT
///
/// ; generic sub-rules: primitives
/// alpha-numeric = ALPHA / DIGIT
/// POS-DIGIT = %x31-39 ; 1 - 9
/// decimal-uchar = DIGIT
///                 / POS-DIGIT DIGIT
///                 / ("1" 2(DIGIT))
///                 / ("2" ("0"/"1"/"2"/"3"/"4") DIGIT)
///                 / ("2" "5" ("0"/"1"/"2"/"3"/"4"/"5"))
///
/// ; external references:
/// ALPHA = <ALPHA definition from RFC 5234>
/// DIGIT = <DIGIT definition from RFC 5234>
/// CRLF = <CRLF definition from RFC 5234>
/// HEXDIG = <HEXDIG definition from RFC 5234>
/// SP = <SP definition from RFC 5234>
/// VCHAR = <VCHAR definition from RFC 5234>
/// URI-reference = <URI-reference definition from RFC 3986>
/// addr-spec = <addr-spec definition from RFC 5322>
///
#[derive(Debug, Default)]
enum SessionDescriptionReadState {
    #[default]
    Version,
    Origin,
    SessionName,
    SessionInformation,
    SessionUri,
    SessionEmail,
    SessionPhone,
    SessionConnection,
    SessionBandwidth,
    SessionTime,
    SessionKey,
    SessionAttribute,
    MediaField,
    MediaInformation,
    MediaConnection,
    MediaBandwidth,
    MediaKey,
    MediaAttribute,
    Finished,
}
pub struct SessionDescriptionReader {
    session_description: SessionDescription,
    read_state: SessionDescriptionReadState,
}
impl SessionDescriptionReader {
    pub fn new() -> Self {
        Self {
            session_description: Default::default(),
            read_state: Default::default(),
        }
    }
    pub fn read_from(mut self, text: &str) -> SDPResult<SessionDescription> {
        if text.is_empty() {
            return Err(SDPError::InvalidPayload(format!(
                "payload is empty: {}",
                text
            )));
        }
        let lines: Vec<&str> = text
            .split(LF)
            .map(|item| item.trim())
            .filter(|item| !item.is_empty())
            .collect();
        if lines.len() < 4 {
            return Err(SDPError::InvalidPayload(format!(
                "too few lines: {}, {}",
                lines.len(),
                text
            )));
        }
        if lines.iter().any(|line| line.len() < 3) {
            return Err(SDPError::InvalidPayload(format!(
                "too short lines found in payload, {}",
                text
            )));
        }

        let trimmed_text = lines.join(CRLF) + CRLF;
        let mut reader = Cursor::new(trimmed_text.as_bytes());
        loop {
            match self.read_state {
                SessionDescriptionReadState::Version => self.read_version_line(&mut reader)?,
                SessionDescriptionReadState::Origin => self.read_origin_line(&mut reader)?,
                SessionDescriptionReadState::SessionName => {
                    self.read_session_name_line(&mut reader)?;
                }
                SessionDescriptionReadState::SessionInformation => {
                    self.read_information_line(&mut reader, false)?;
                }
                SessionDescriptionReadState::SessionUri => {
                    self.read_session_uri_line(&mut reader)?;
                }
                SessionDescriptionReadState::SessionEmail => {
                    self.read_session_email_line(&mut reader)?;
                }
                SessionDescriptionReadState::SessionPhone => {
                    self.read_session_phone_line(&mut reader)?;
                }
                SessionDescriptionReadState::SessionConnection => {
                    self.read_connection_line(&mut reader, false)?;
                }
                SessionDescriptionReadState::SessionBandwidth => {
                    self.read_bandwidth_line(&mut reader, false)?;
                }
                SessionDescriptionReadState::SessionTime => {
                    self.read_session_time_line(&mut reader)?;
                }
                SessionDescriptionReadState::SessionKey => {
                    self.read_key_line(&mut reader, false)?;
                }
                SessionDescriptionReadState::SessionAttribute => {
                    self.read_attribute_line(&mut reader, false)?;
                }
                SessionDescriptionReadState::MediaField => {
                    self.read_media_field(&mut reader)?;
                }
                SessionDescriptionReadState::MediaInformation => {
                    self.read_information_line(&mut reader, true)?;
                }
                SessionDescriptionReadState::MediaConnection => {
                    self.read_connection_line(&mut reader, true)?;
                }
                SessionDescriptionReadState::MediaBandwidth => {
                    self.read_bandwidth_line(&mut reader, true)?;
                }
                SessionDescriptionReadState::MediaKey => {
                    self.read_key_line(&mut reader, true)?;
                }
                SessionDescriptionReadState::MediaAttribute => {
                    self.read_attribute_line(&mut reader, true)?;
                }
                SessionDescriptionReadState::Finished => break,
            }
        }
        Ok(self.session_description)
    }

    fn read_line_type(reader: &mut Cursor<&[u8]>) -> SDPResult<[u8; 2]> {
        let mut result = [0_u8; 2];
        reader.read_exact(&mut result)?;
        Ok(result)
    }

    fn take_line_type(reader: &mut Cursor<&[u8]>) -> SDPResult<[u8; 2]> {
        let result = Self::read_line_type(reader)?;
        reader.seek_relative(-2)?;
        Ok(result)
    }

    fn read_line(reader: &mut Cursor<&[u8]>) -> SDPResult<String> {
        let mut result = String::new();
        reader.read_line(&mut result)?;
        if !result.ends_with(LF) {
            return Err(SDPError::InvalidPayload(format!(
                "invalid line not ends with LF: {}",
                result
            )));
        }
        Ok(result.trim().into())
    }

    fn expect_line_type(reader: &mut Cursor<&[u8]>, expected: &[u8; 2]) -> SDPResult<()> {
        let line_type = &Self::read_line_type(reader)?;
        if expected != line_type {
            return Err(SDPError::SyntaxError(format!(
                "expect line type: {}{}, got: {}{}",
                std::ascii::escape_default(expected[0]),
                std::ascii::escape_default(expected[1]),
                std::ascii::escape_default(line_type[0]),
                std::ascii::escape_default(line_type[1])
            )));
        }
        Ok(())
    }

    fn read_version_line(&mut self, reader: &mut Cursor<&[u8]>) -> SDPResult<()> {
        Self::expect_line_type(reader, b"v=")?;
        let version = Self::read_line(reader)?;
        self.session_description.version = version.parse::<u32>().map_err(|err| {
            SDPError::SyntaxError(format!("parse version failed: {}, {}", version, err))
        })?;

        self.read_state = SessionDescriptionReadState::Origin;
        Ok(())
    }

    fn read_origin_line(&mut self, reader: &mut Cursor<&[u8]>) -> SDPResult<()> {
        Self::expect_line_type(reader, b"o=")?;
        let line = Self::read_line(reader)?;
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() != 6 {
            return Err(SDPError::SyntaxError(format!(
                "invalid origin line, fields count is not 6: {}",
                line,
            )));
        }

        self.session_description.origin.user_name = fields[0].to_owned();
        self.session_description.origin.session_id = fields[1].parse::<_>().map_err(|err| {
            SDPError::SyntaxError(format!("parse session id failed: {}, {}", fields[1], err))
        })?;
        self.session_description.origin.session_version =
            fields[2].parse::<_>().map_err(|err| {
                SDPError::SyntaxError(format!(
                    "parse session version failed: {}, {}",
                    fields[2], err
                ))
            })?;
        self.session_description.origin.net_type = fields[3].into();
        self.session_description.origin.addr_type = fields[4].into();
        self.session_description.origin.unicast_address = fields[5].to_owned();

        self.read_state = SessionDescriptionReadState::SessionName;
        Ok(())
    }

    fn read_next_line_type(
        &mut self,
        reader: &mut Cursor<&[u8]>,
        expected_types: &[&[u8; 2]],
        reading_media: bool,
    ) -> SDPResult<()> {
        if !reader.has_remaining() || reader.remaining() < 2 {
            self.read_state = SessionDescriptionReadState::Finished;
            return Ok(());
        }
        let line_type = Self::take_line_type(reader)?;
        if &line_type == b"\r\n" {
            self.read_state = SessionDescriptionReadState::Finished;
            return Ok(());
        }
        if !expected_types.contains(&&line_type) {
            return Err(SDPError::SyntaxError(format!(
                "invalid line type: {}{} in current context",
                std::ascii::escape_default(line_type[0]),
                std::ascii::escape_default(line_type[1])
            )));
        }

        match &line_type {
            b"i=" => {
                self.read_state = if reading_media {
                    SessionDescriptionReadState::MediaInformation
                } else {
                    SessionDescriptionReadState::SessionInformation
                }
            }
            b"u=" => self.read_state = SessionDescriptionReadState::SessionUri,
            b"e=" => self.read_state = SessionDescriptionReadState::SessionEmail,
            b"p=" => self.read_state = SessionDescriptionReadState::SessionPhone,
            b"c=" => {
                self.read_state = if reading_media {
                    SessionDescriptionReadState::MediaConnection
                } else {
                    SessionDescriptionReadState::SessionConnection
                }
            }
            b"b=" => {
                self.read_state = if reading_media {
                    SessionDescriptionReadState::MediaBandwidth
                } else {
                    SessionDescriptionReadState::SessionBandwidth
                }
            }
            b"t=" => self.read_state = SessionDescriptionReadState::SessionTime,
            b"k=" => {
                self.read_state = if reading_media {
                    SessionDescriptionReadState::MediaKey
                } else {
                    SessionDescriptionReadState::SessionKey
                }
            }
            b"a=" => {
                self.read_state = if reading_media {
                    SessionDescriptionReadState::MediaAttribute
                } else {
                    SessionDescriptionReadState::SessionAttribute
                }
            }
            b"m=" => self.read_state = SessionDescriptionReadState::MediaField,
            t => {
                return Err(SDPError::SyntaxError(format!(
                    "invalid line type: {}{} in current context",
                    std::ascii::escape_default(t[0]),
                    std::ascii::escape_default(t[1])
                )));
            }
        }
        Ok(())
    }

    fn read_session_name_line(&mut self, reader: &mut Cursor<&[u8]>) -> SDPResult<()> {
        Self::expect_line_type(reader, b"s=")?;
        let session_name = Self::read_line(reader)?;
        self.session_description.session_name = session_name;

        self.read_next_line_type(
            reader,
            &[b"i=", b"u=", b"e=", b"p=", b"c=", b"b=", b"t="],
            false,
        )?;

        Ok(())
    }

    fn read_information_line(
        &mut self,
        reader: &mut Cursor<&[u8]>,
        reading_media: bool,
    ) -> SDPResult<()> {
        Self::expect_line_type(reader, b"i=")?;
        let information = Self::read_line(reader)?;
        if reading_media {
            if let Some(media_info) = self.session_description.media_description.last_mut() {
                media_info.media_title = Some(information);
            } else {
                return Err(SDPError::SyntaxError(
                    "got information in media mode while there is no media info".to_owned(),
                ));
            }
        } else {
            self.session_description.session_information = Some(information);
        }

        if reading_media {
            self.read_next_line_type(reader, &[b"m=", b"c=", b"b=", b"k=", b"a="], true)?;
        } else {
            self.read_next_line_type(reader, &[b"u=", b"e=", b"p=", b"c=", b"b=", b"t="], false)?;
        }
        Ok(())
    }

    fn read_session_uri_line(&mut self, reader: &mut Cursor<&[u8]>) -> SDPResult<()> {
        Self::expect_line_type(reader, b"u=")?;
        let url_str = Self::read_line(reader)?;
        self.session_description.uri = Some(Url::parse(&url_str)?);

        self.read_next_line_type(reader, &[b"e=", b"p=", b"c=", b"b=", b"t="], false)?;
        Ok(())
    }

    fn read_session_email_line(&mut self, reader: &mut Cursor<&[u8]>) -> SDPResult<()> {
        Self::expect_line_type(reader, b"e=")?;
        let email = Self::read_line(reader)?;
        self.session_description.email_address.push(email);

        self.read_next_line_type(reader, &[b"e=", b"p=", b"c=", b"b=", b"t="], false)?;
        Ok(())
    }

    fn read_session_phone_line(&mut self, reader: &mut Cursor<&[u8]>) -> SDPResult<()> {
        Self::expect_line_type(reader, b"p=")?;
        let phone = Self::read_line(reader)?;
        self.session_description.phone_number.push(phone);

        self.read_next_line_type(reader, &[b"p=", b"c=", b"b=", b"t="], false)?;
        Ok(())
    }

    fn parse_connection_information(text: &str) -> SDPResult<SDPConnectionInformation> {
        let fields: Vec<&str> = text.split_whitespace().collect();
        if fields.len() < 3 {
            return Err(SDPError::SyntaxError(format!(
                "invalid connection information line, too few fields: {}",
                text
            )));
        }

        let mut result = SDPConnectionInformation {
            net_type: fields[0].into(),
            addr_type: fields[1].into(),
            connection_address: Default::default(),
        };

        if !fields[2].contains('/') {
            result.connection_address.address = fields[2].to_owned();
            return Ok(result);
        }
        let connection_address_fields: Vec<&str> = fields[2].split('/').collect();
        if connection_address_fields.len() < 2 || connection_address_fields.len() > 3 {
            return Err(SDPError::SyntaxError(format!(
                "invalid connection address field: {}, full line: {}",
                fields[2], text
            )));
        }

        result.connection_address.address = connection_address_fields[0].to_owned();
        if result
            .connection_address
            .address
            .parse::<Ipv4Addr>()
            .is_ok()
        {
            let ttl: u8 = connection_address_fields[1].parse().map_err(|err| {
                SDPError::SyntaxError(format!(
                    "parse connection ttl failed: {}, {}",
                    connection_address_fields[1], err
                ))
            })?;
            result.connection_address.ttl = Some(ttl.into());
            if connection_address_fields.len() == 3 {
                let number_addr: u64 = connection_address_fields[2].parse().map_err(|err| {
                    SDPError::SyntaxError(format!(
                        "parse connection address range failed: {}, {}",
                        connection_address_fields[2], err
                    ))
                })?;
                result.connection_address.range = Some(number_addr);
            }
        } else if result
            .connection_address
            .address
            .parse::<Ipv6Addr>()
            .is_ok()
        {
            if connection_address_fields.len() == 3 {
                return Err(SDPError::SyntaxError(format!(
                    "invalid connection address field: {}, full line: {}",
                    fields[3], text
                )));
            }

            let number_addr: u64 = fields[1].parse().map_err(|err| {
                SDPError::SyntaxError(format!(
                    "parse connection address range failed: {}, {}",
                    fields[1], err
                ))
            })?;
            result.connection_address.range = Some(number_addr);
        }

        Ok(result)
    }

    fn read_connection_line(
        &mut self,
        reader: &mut Cursor<&[u8]>,
        reading_media: bool,
    ) -> SDPResult<()> {
        Self::expect_line_type(reader, b"c=")?;
        let line = Self::read_line(reader)?;
        let connection_info = Self::parse_connection_information(&line)?;

        if reading_media {
            if let Some(media_info) = self.session_description.media_description.last_mut() {
                media_info.connection_information.push(connection_info);
            } else {
                return Err(SDPError::SyntaxError(
                    "got connection information line in media mode while there is no media info"
                        .to_owned(),
                ));
            }
        } else {
            self.session_description.connection_information = Some(connection_info);
        }

        if reading_media {
            self.read_next_line_type(reader, &[b"m=", b"b=", b"k=", b"a="], true)?;
        } else {
            self.read_next_line_type(reader, &[b"b=", b"t="], false)?;
        }
        Ok(())
    }

    fn parse_bandwidth(text: &str) -> SDPResult<SDPBandWidthInformation> {
        let bandwidth_fields: Vec<&str> = text.split(':').collect();
        if bandwidth_fields.len() != 2 {
            return Err(SDPError::SyntaxError(format!(
                "invalid bandwidth line: {}",
                text
            )));
        }

        let bandwidth: u64 = bandwidth_fields[1].parse().map_err(|err| {
            SDPError::SyntaxError(format!(
                "parse bandwidth failed: {}, {}",
                bandwidth_fields[1], err
            ))
        })?;
        Ok(SDPBandWidthInformation {
            bw_type: bandwidth_fields[0].to_owned(),
            bandwidth,
        })
    }

    fn read_bandwidth_line(
        &mut self,
        reader: &mut Cursor<&[u8]>,
        reading_media: bool,
    ) -> SDPResult<()> {
        Self::expect_line_type(reader, b"b=")?;
        let line = Self::read_line(reader)?;
        let bandwidth = Self::parse_bandwidth(&line)?;

        if reading_media {
            if let Some(media_info) = self.session_description.media_description.last_mut() {
                media_info.bandwidth.push(bandwidth);
            } else {
                return Err(SDPError::SyntaxError(
                    "got bandwidth information line in media mode while there is no media info"
                        .to_owned(),
                ));
            }
        } else {
            self.session_description
                .bandwidth_information
                .push(bandwidth);
        }

        if reading_media {
            self.read_next_line_type(reader, &[b"m=", b"b=", b"k=", b"a="], true)?;
        } else {
            self.read_next_line_type(reader, &[b"b=", b"t="], false)?;
        }
        Ok(())
    }

    fn parse_time(text: &str) -> SDPResult<SDPTimeInformation> {
        let time_fields: Vec<&str> = text.split_whitespace().collect();
        if time_fields.len() != 2 {
            return Err(SDPError::SyntaxError(format!(
                "invalid time line: {}",
                text
            )));
        }

        Ok(SDPTimeInformation {
            start_time: time_fields[0].parse().map_err(|err| {
                SDPError::SyntaxError(format!(
                    "parse start time failed: {}, {}",
                    time_fields[0], err
                ))
            })?,
            stop_time: time_fields[1].parse().map_err(|err| {
                SDPError::SyntaxError(format!(
                    "parse stop time failed: {}, {}",
                    time_fields[1], err
                ))
            })?,
            repeat_times: Vec::new(),
        })
    }

    // 1h -> 3600s
    fn parse_typed_time(text: &str) -> SDPResult<i64> {
        if text.is_empty() {
            return Err(SDPError::SyntaxError(
                "empty str should not be typed time".to_owned(),
            ));
        }
        const TYPED_TIME_OFFSETS: [char; 4] = ['d', 'h', 'm', 's'];
        let last_char = text.chars().last().unwrap();
        if !TYPED_TIME_OFFSETS.contains(&last_char) {
            return text.parse().map_err(|err| {
                SDPError::SyntaxError(format!("parse time field failed: {}, {}", text, err))
            });
        }

        let unit_number: i64 = text
            .strip_suffix(TYPED_TIME_OFFSETS)
            .unwrap()
            .parse()
            .map_err(|err| {
                SDPError::SyntaxError(format!("parse time unit number failed: {}, {}", text, err))
            })?;

        match last_char {
            'd' => Ok(unit_number.checked_mul(86400).ok_or_else(|| {
                SDPError::IntegerOverflow(format!(
                    "multiply {} with {} as i64 overflow",
                    unit_number, 86400
                ))
            })?),
            'h' => Ok(unit_number.checked_mul(3600).ok_or_else(|| {
                SDPError::IntegerOverflow(format!(
                    "multiply {} with {} as i64 overflow",
                    unit_number, 3600
                ))
            })?),
            'm' => Ok(unit_number.checked_mul(60).ok_or_else(|| {
                SDPError::IntegerOverflow(format!(
                    "multiply {} with {} as i64 overflow",
                    unit_number, 60
                ))
            })?),
            's' => Ok(unit_number),
            _ => unreachable!(),
        }
    }

    fn parse_repeat(text: &str) -> SDPResult<SDPRepeatTime> {
        let repeat_fields: Vec<&str> = text.split_whitespace().collect();
        if repeat_fields.len() < 2 {
            return Err(SDPError::SyntaxError(format!(
                "invalid repeat description line: {}",
                text
            )));
        }

        let mut offsets: Vec<i64> = Vec::with_capacity(repeat_fields.len() - 2);
        repeat_fields[2..].iter().try_for_each(|item| {
            let value = Self::parse_typed_time(item)?;
            offsets.push(value);
            Ok::<(), SDPError>(())
        })?;

        Ok(SDPRepeatTime {
            interval: Self::parse_typed_time(repeat_fields[0])?,
            duration: Self::parse_typed_time(repeat_fields[1])?,
            offsets,
            time_zone_adjustment: Vec::new(),
        })
    }

    fn parse_zone(text: &str) -> SDPResult<Vec<SDPTimeZoneAdjustment>> {
        let zone_fields: Vec<&str> = text.split_whitespace().collect();
        if zone_fields.len() % 2 != 0 {
            return Err(SDPError::SyntaxError(format!(
                "invalid zone line: {}",
                text
            )));
        }

        let mut result: Vec<SDPTimeZoneAdjustment> = Vec::with_capacity(zone_fields.len() / 2);
        zone_fields.chunks(2).try_for_each(|item| {
            let adjust: i64 = item[0].parse().map_err(|err| {
                SDPError::SyntaxError(format!(
                    "parse time zone adjustment failed: {}, {}",
                    item[0], err
                ))
            })?;
            result.push(SDPTimeZoneAdjustment {
                adjustment_time: adjust,
                offset: Self::parse_typed_time(item[1])?,
            });
            Ok::<(), SDPError>(())
        })?;
        Ok(result)
    }

    fn read_time_information(reader: &mut Cursor<&[u8]>) -> SDPResult<SDPTimeInformation> {
        Self::expect_line_type(reader, b"t=")?;
        let line = Self::read_line(reader)?;
        let mut time = Self::parse_time(&line)?;

        while let Ok(next_line_type) = Self::take_line_type(reader) {
            match &next_line_type {
                b"r=" => {
                    reader.seek_relative(2)?;
                    let repeat_line = Self::read_line(reader)?;
                    let repeat = Self::parse_repeat(&repeat_line)?;
                    time.repeat_times.push(repeat);
                }
                b"z=" => {
                    if time.repeat_times.is_empty() {
                        return Err(SDPError::SyntaxError(
                            "got zone line without time information or repeat lines".to_owned(),
                        ));
                    }
                    if !time
                        .repeat_times
                        .last()
                        .unwrap()
                        .time_zone_adjustment
                        .is_empty()
                    {
                        return Err(SDPError::SyntaxError(
                            "got multiple zone line for one repeat line".to_owned(),
                        ));
                    }
                    reader.seek_relative(2)?;
                    let zone_line = Self::read_line(reader)?;
                    let zone = Self::parse_zone(&zone_line)?;
                    time.repeat_times.last_mut().unwrap().time_zone_adjustment = zone
                }
                _ => break,
            }
        }
        Ok(time)
    }

    fn read_session_time_line(&mut self, reader: &mut Cursor<&[u8]>) -> SDPResult<()> {
        let time = Self::read_time_information(reader)?;
        self.session_description.time_information.push(time);

        self.read_next_line_type(reader, &[b"t=", b"k=", b"a=", b"m="], false)?;
        Ok(())
    }

    fn parse_key(text: &str) -> SDPResult<SDPEncryptionKeys> {
        let key_fields: Vec<&str> = text.split(':').collect();
        if key_fields.len() == 1 {
            return Ok(SDPEncryptionKeys {
                method: key_fields[0].to_owned(),
                key: None,
            });
        }
        if key_fields.len() == 2 {
            return Ok(SDPEncryptionKeys {
                method: key_fields[0].to_owned(),
                key: Some(key_fields[1].to_owned()),
            });
        }

        Err(SDPError::SyntaxError(format!("invalid key line: {}", text)))
    }

    fn read_key_line(&mut self, reader: &mut Cursor<&[u8]>, reading_media: bool) -> SDPResult<()> {
        Self::expect_line_type(reader, b"k=")?;
        let line = Self::read_line(reader)?;
        let key = Self::parse_key(&line)?;

        if reading_media {
            if let Some(media_info) = self.session_description.media_description.last_mut() {
                media_info.encryption_key = Some(key);
            } else {
                return Err(SDPError::SyntaxError(
                    "got key line in media mode while there is not media info".to_owned(),
                ));
            }
        } else {
            self.session_description.encryption_keys = Some(key);
        }

        if reading_media {
            self.read_next_line_type(reader, &[b"m=", b"a="], true)?;
        } else {
            self.read_next_line_type(reader, &[b"a=", b"m="], false)?;
        }
        Ok(())
    }

    fn parse_attribute(text: &str) -> SDPResult<SDPAttribute> {
        text.parse()
        // let fields: Vec<&str> = text.split(':').collect();
        // if fields.is_empty() {
        //     return Err(SDPError::SyntaxError(format!(
        //         "invalid attribute line: {}",
        //         text
        //     )));
        // }
        // if fields.len() == 1 {
        //     return Ok(SDPTrivialAttribute {
        //         name: fields[0].to_owned(),
        //         value: None,
        //     });
        // }

        // Ok(SDPTrivialAttribute {
        //     name: fields[0].to_owned(),
        //     value: Some(fields[1..].join(":").to_owned()),
        // })
    }

    fn read_attribute_line(
        &mut self,
        reader: &mut Cursor<&[u8]>,
        reading_media: bool,
    ) -> SDPResult<()> {
        Self::expect_line_type(reader, b"a=")?;
        let line = Self::read_line(reader)?;
        let attribute = Self::parse_attribute(&line)?;

        if reading_media {
            if let Some(media_info) = self.session_description.media_description.last_mut() {
                media_info.attributes.push(attribute);
            } else {
                return Err(SDPError::SyntaxError(
                    "got attribute line in media mode while there is no media info".to_owned(),
                ));
            }
        } else {
            self.session_description.attributes.push(attribute);
        }

        if reading_media {
            self.read_next_line_type(reader, &[b"a=", b"m="], true)?;
        } else {
            self.read_next_line_type(reader, &[b"a=", b"m="], false)?;
        }
        Ok(())
    }

    fn read_media_field(&mut self, reader: &mut Cursor<&[u8]>) -> SDPResult<()> {
        Self::expect_line_type(reader, b"m=")?;
        let line = Self::read_line(reader)?;
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 4 {
            return Err(SDPError::SyntaxError(format!(
                "invalid media field line: {}",
                line
            )));
        }

        let mut media: SDPMediaDescription = Default::default();
        media.media_line.media_type = fields[0].into();
        media.media_line.port = fields[1].try_into()?;
        media.media_line.protocol = fields[2].into();
        media.media_line.format = fields[3..].iter().map(|item| item.to_string()).collect();
        self.session_description.media_description.push(media);

        self.read_next_line_type(reader, &[b"m=", b"i=", b"c=", b"b=", b"k=", b"a="], true)?;

        Ok(())
    }
}

impl Default for SessionDescriptionReader {
    fn default() -> Self {
        Self::new()
    }
}
