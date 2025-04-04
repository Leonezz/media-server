/// taken from webrtc-rs: https://github.com/webrtc-rs/webrtc
mod tests {

    use url::Url;

    use crate::{
        attributes::{SDPAttribute, SDPTrivialAttribute},
        errors::SDPResult,
        reader::SessionDescriptionReader,
        session::{
            SDPAddress, SDPBandWidthInformation, SDPConnectionInformation, SDPEncryptionKeys,
            SDPMediaDescription, SDPMediaLine, SDPOrigin, SDPRangedPort, SDPRepeatTime,
            SDPTimeInformation, SDPTimeZoneAdjustment, SessionDescription,
        },
    };

    const CANONICAL_MARSHAL_SDP: &str = "v=0\r\n\
  o=jdoe 2890844526 2890842807 IN IP4 10.47.16.5\r\n\
  s=SDP Seminar\r\n\
  i=A Seminar on the session description protocol\r\n\
  u=http://www.example.com/seminars/sdp.pdf\r\n\
  e=j.doe@example.com (Jane Doe)\r\n\
  p=+1 617 555-6011\r\n\
  c=IN IP4 224.2.17.12/127\r\n\
  b=X-YZ:128\r\n\
  b=AS:12345\r\n\
  t=2873397496 2873404696\r\n\
  t=3034423619 3042462419\r\n\
  r=604800 3600 0 90000\r\n\
  z=2882844526 -3600 2898848070 0\r\n\
  k=prompt\r\n\
  a=candidate:0 1 UDP 2113667327 203.0.113.1 54400 typ host\r\n\
  a=recvonly\r\n\
  m=audio 49170 RTP/AVP 0\r\n\
  i=Vivamus a posuere nisl\r\n\
  c=IN IP4 203.0.113.1\r\n\
  b=X-YZ:128\r\n\
  k=prompt\r\n\
  a=sendrecv\r\n\
  m=video 51372 RTP/AVP 99\r\n\
  a=rtpmap:99 h263-1998/90000\r\n";

    #[test]
    fn test_unmarshal_marshal() {
        let input = CANONICAL_MARSHAL_SDP;
        let sdp = SessionDescriptionReader::new().read_from(input).unwrap();
        let output = format!("{}", sdp);
        assert_eq!(output, input);
    }

    #[test]
    fn test_marshal() -> SDPResult<()> {
        let sd = SessionDescription {
            version: 0,
            origin: SDPOrigin {
                user_name: "jdoe".to_string(),
                session_id: 2890844526,
                session_version: 2890842807,
                net_type: "IN".into(),
                addr_type: "IP4".into(),
                unicast_address: "10.47.16.5".to_string(),
            },
            session_name: "SDP Seminar".to_string(),
            session_information: Some("A Seminar on the session description protocol".to_string()),
            uri: Some(Url::parse("http://www.example.com/seminars/sdp.pdf")?),
            email_address: vec!["j.doe@example.com (Jane Doe)".to_owned()],
            phone_number: vec!["+1 617 555-6011".to_owned()],
            connection_information: Some(SDPConnectionInformation {
                net_type: "IN".into(),
                addr_type: "IP4".into(),
                connection_address: SDPAddress {
                    address: "224.2.17.12".to_string(),
                    ttl: Some(127),
                    range: None,
                },
            }),
            bandwidth_information: vec![
                SDPBandWidthInformation {
                    bw_type: "X-YZ".parse()?,
                    bandwidth: 128,
                },
                SDPBandWidthInformation {
                    bw_type: "AS".parse()?,
                    bandwidth: 12345,
                },
            ],
            time_information: vec![
                SDPTimeInformation {
                    start_time: 2873397496,
                    stop_time: 2873404696,
                    repeat_times: vec![],
                },
                SDPTimeInformation {
                    start_time: 3034423619,
                    stop_time: 3042462419,

                    repeat_times: vec![SDPRepeatTime {
                        interval: 604800,
                        duration: 3600,
                        offsets: vec![0, 90000],
                        time_zone_adjustment: vec![
                            SDPTimeZoneAdjustment {
                                adjustment_time: 2882844526,
                                offset: -3600,
                            },
                            SDPTimeZoneAdjustment {
                                adjustment_time: 2898848070,
                                offset: 0,
                            },
                        ],
                    }],
                },
            ],
            encryption_keys: Some(SDPEncryptionKeys {
                method: "prompt".to_string(),
                key: None,
            }),
            attributes: vec![
                SDPAttribute::Trivial(SDPTrivialAttribute {
                    name: "candidate".to_string(),
                    value: Some("0 1 UDP 2113667327 203.0.113.1 54400 typ host".to_string()),
                }),
                SDPAttribute::Trivial(SDPTrivialAttribute {
                    name: "recvonly".to_string(),
                    value: None,
                }),
            ],
            media_description: vec![
                SDPMediaDescription {
                    media_line: SDPMediaLine {
                        media_type: "audio".into(),
                        port: SDPRangedPort {
                            port: 49170,
                            range: None,
                        },
                        protocol: "RTP/AVP".into(),
                        format: vec!["0".to_string()],
                    },
                    media_title: Some("Vivamus a posuere nisl".to_string()),
                    connection_information: vec![SDPConnectionInformation {
                        net_type: "IN".into(),
                        addr_type: "IP4".into(),
                        connection_address: SDPAddress {
                            address: "203.0.113.1".to_string(),
                            ttl: None,
                            range: None,
                        },
                    }],
                    bandwidth: vec![SDPBandWidthInformation {
                        bw_type: "X-YZ".parse()?,
                        bandwidth: 128,
                    }],
                    encryption_key: Some(SDPEncryptionKeys {
                        method: "prompt".to_string(),
                        key: None,
                    }),
                    attributes: vec![SDPAttribute::Trivial(SDPTrivialAttribute {
                        name: "sendrecv".to_string(),
                        value: None,
                    })],
                },
                SDPMediaDescription {
                    media_line: SDPMediaLine {
                        media_type: "video".into(),
                        port: SDPRangedPort {
                            port: 51372,
                            range: None,
                        },
                        protocol: "RTP/AVP".into(),
                        format: vec!["99".to_string()],
                    },
                    media_title: None,
                    connection_information: vec![],
                    bandwidth: vec![],
                    encryption_key: None,
                    attributes: vec![SDPAttribute::Trivial(SDPTrivialAttribute {
                        name: "rtpmap".to_string(),
                        value: Some("99 h263-1998/90000".to_string()),
                    })],
                },
            ],
        };

        let actual = format!("{}", sd);
        assert!(
            actual == CANONICAL_MARSHAL_SDP,
            "error:\n\nEXPECTED:\n{CANONICAL_MARSHAL_SDP}\nACTUAL:\n{actual}\n!!!!\n"
        );

        Ok(())
    }

    #[allow(dead_code)]
    const BASE_SDP: &str = "v=0\r\n\
o=jdoe 2890844526 2890842807 IN IP4 10.47.16.5\r\n\
s=SDP Seminar\r\n";

    const SESSION_INFORMATION_SDP: &str = "v=0\r\n\
o=jdoe 2890844526 2890842807 IN IP4 10.47.16.5\r\n\
s=SDP Seminar\r\n\
i=A Seminar on the session description protocol\r\n\
t=3034423619 3042462419\r\n";

    // https://tools.ietf.org/html/rfc4566#section-5
    // Parsers SHOULD be tolerant and also accept records terminated
    // with a single newline character.
    const SESSION_INFORMATION_SDPLFONLY: &str = "v=0\n\
o=jdoe 2890844526 2890842807 IN IP4 10.47.16.5\n\
s=SDP Seminar\n\
i=A Seminar on the session description protocol\n\
t=3034423619 3042462419\n";

    // SessionInformationSDPCROnly = "v=0\r" +
    // 	"o=jdoe 2890844526 2890842807 IN IP4 10.47.16.5\r" +
    // 	"s=SDP Seminar\r"
    // 	"i=A Seminar on the session description protocol\r" +
    // 	"t=3034423619 3042462419\r"

    // Other SDP parsers (e.g. one in VLC media player) allow
    // empty lines.
    const SESSION_INFORMATION_SDPEXTRA_CRLF: &str = "v=0\r\n\
o=jdoe 2890844526 2890842807 IN IP4 10.47.16.5\r\n\
\r\n\
s=SDP Seminar\r\n\
\r\n\
i=A Seminar on the session description protocol\r\n\
\r\n\
t=3034423619 3042462419\r\n\
\r\n";

    const URI_SDP: &str = "v=0\r\n\
o=jdoe 2890844526 2890842807 IN IP4 10.47.16.5\r\n\
s=SDP Seminar\r\n\
u=http://www.example.com/seminars/sdp.pdf\r\n\
t=3034423619 3042462419\r\n";

    const EMAIL_ADDRESS_SDP: &str = "v=0\r\n\
o=jdoe 2890844526 2890842807 IN IP4 10.47.16.5\r\n\
s=SDP Seminar\r\n\
e=j.doe@example.com (Jane Doe)\r\n\
t=3034423619 3042462419\r\n";

    const PHONE_NUMBER_SDP: &str = "v=0\r\n\
o=jdoe 2890844526 2890842807 IN IP4 10.47.16.5\r\n\
s=SDP Seminar\r\n\
p=+1 617 555-6011\r\n\
t=3034423619 3042462419\r\n";

    const SESSION_CONNECTION_INFORMATION_SDP: &str = "v=0\r\n\
o=jdoe 2890844526 2890842807 IN IP4 10.47.16.5\r\n\
s=SDP Seminar\r\n\
c=IN IP4 224.2.17.12/127\r\n\
t=3034423619 3042462419\r\n";

    const SESSION_BANDWIDTH_SDP: &str = "v=0\r\n\
o=jdoe 2890844526 2890842807 IN IP4 10.47.16.5\r\n\
s=SDP Seminar\r\n\
b=X-YZ:128\r\n\
b=AS:12345\r\n\
t=3034423619 3042462419\r\n";
    #[allow(dead_code)]
    const TIMING_SDP: &str = "v=0\r\n\
o=jdoe 2890844526 2890842807 IN IP4 10.47.16.5\r\n\
s=SDP Seminar\r\n\
t=2873397496 2873404696\r\n";

    // Short hand time notation is converted into NTP timestamp format in
    // seconds. Because of that unittest comparisons will fail as the same time
    // will be expressed in different units.
    const REPEAT_TIMES_SDP: &str = "v=0\r\n\
o=jdoe 2890844526 2890842807 IN IP4 10.47.16.5\r\n\
s=SDP Seminar\r\n\
t=2873397496 2873404696\r\n\
r=604800 3600 0 90000\r\n\
r=3d 2h 0 21h\r\n";

    const REPEAT_TIMES_SDPEXPECTED: &str = "v=0\r\n\
o=jdoe 2890844526 2890842807 IN IP4 10.47.16.5\r\n\
s=SDP Seminar\r\n\
t=2873397496 2873404696\r\n\
r=604800 3600 0 90000\r\n\
r=259200 7200 0 75600\r\n";

    const REPEAT_TIMES_OVERFLOW_SDP: &str = "v=0\r\n\
o=jdoe 2890844526 2890842807 IN IP4 10.47.16.5\r\n\
s=SDP Seminar\r\n\
t=2873397496 2873404696\r\n\
r=604800 3600 0 90000\r\n\
r=106751991167301d 2h 0 21h\r\n";

    const REPEAT_TIMES_SDPEXTRA_CRLF: &str = "v=0\r\n\
o=jdoe 2890844526 2890842807 IN IP4 10.47.16.5\r\n\
s=SDP Seminar\r\n\
t=2873397496 2873404696\r\n\
r=604800 3600 0 90000\r\n\
r=259200 7200 0 75600\r\n\
\r\n";

    // The expected value looks a bit different for the same reason as mentioned
    // above regarding RepeatTimes.
    const TIME_ZONES_SDP: &str = "v=0\r\n\
o=jdoe 2890844526 2890842807 IN IP4 10.47.16.5\r\n\
s=SDP Seminar\r\n\
t=2873397496 2873404696\r\n\
r=2882844526 -1h 2898848070 0\r\n";

    const TIME_ZONES_SDPEXPECTED: &str = "v=0\r\n\
o=jdoe 2890844526 2890842807 IN IP4 10.47.16.5\r\n\
s=SDP Seminar\r\n\
t=2873397496 2873404696\r\n\
r=2882844526 -3600 2898848070 0\r\n";
    #[allow(dead_code)]
    const TIME_ZONES_SDP2: &str = "v=0\r\n\
o=jdoe 2890844526 2890842807 IN IP4 10.47.16.5\r\n\
s=SDP Seminar\r\n\
t=2873397496 2873404696\r\n\
z=2882844526 -3600 2898848070 0\r\n";
    #[allow(dead_code)]
    const TIME_ZONES_SDP2EXTRA_CRLF: &str = "v=0\r\n\
o=jdoe 2890844526 2890842807 IN IP4 10.47.16.5\r\n\
s=SDP Seminar\r\n\
t=2873397496 2873404696\r\n\
z=2882844526 -3600 2898848070 0\r\n\
\r\n";

    const SESSION_ENCRYPTION_KEY_SDP: &str = "v=0\r\n\
o=jdoe 2890844526 2890842807 IN IP4 10.47.16.5\r\n\
s=SDP Seminar\r\n\
t=2873397496 2873404696\r\n\
k=prompt\r\n";

    const SESSION_ENCRYPTION_KEY_SDPEXTRA_CRLF: &str = "v=0\r\n\
o=jdoe 2890844526 2890842807 IN IP4 10.47.16.5\r\n\
s=SDP Seminar\r\n\
t=2873397496 2873404696\r\n\
k=prompt\r\n
\r\n";

    const SESSION_ATTRIBUTES_SDP: &str = "v=0\r\n\
o=jdoe 2890844526 2890842807 IN IP4 10.47.16.5\r\n\
s=SDP Seminar\r\n\
t=2873397496 2873404696\r\n\
a=rtpmap:96 opus/48000\r\n";

    const MEDIA_NAME_SDP: &str = "v=0\r\n\
o=jdoe 2890844526 2890842807 IN IP4 10.47.16.5\r\n\
s=SDP Seminar\r\n\
t=2873397496 2873404696\r\n\
m=video 51372 RTP/AVP 99\r\n\
m=audio 54400 RTP/SAVPF 0 96\r\n";

    const MEDIA_NAME_SDPEXTRA_CRLF: &str = "v=0\r\n\
o=jdoe 2890844526 2890842807 IN IP4 10.47.16.5\r\n\
s=SDP Seminar\r\n\
t=2873397496 2873404696\r\n\
m=video 51372 RTP/AVP 99\r\n\
m=audio 54400 RTP/SAVPF 0 96\r\n
\r\n";

    const MEDIA_TITLE_SDP: &str = "v=0\r\n\
o=jdoe 2890844526 2890842807 IN IP4 10.47.16.5\r\n\
s=SDP Seminar\r\n\
t=2873397496 2873404696\r\n\
m=video 51372 RTP/AVP 99\r\n\
m=audio 54400 RTP/SAVPF 0 96\r\n\
i=Vivamus a posuere nisl\r\n";

    const MEDIA_CONNECTION_INFORMATION_SDP: &str = "v=0\r\n\
o=jdoe 2890844526 2890842807 IN IP4 10.47.16.5\r\n\
s=SDP Seminar\r\n\
t=2873397496 2873404696\r\n\
m=video 51372 RTP/AVP 99\r\n\
m=audio 54400 RTP/SAVPF 0 96\r\n\
c=IN IP4 203.0.113.1\r\n";

    const MEDIA_CONNECTION_INFORMATION_SDPEXTRA_CRLF: &str = "v=0\r\n\
o=jdoe 2890844526 2890842807 IN IP4 10.47.16.5\r\n\
s=SDP Seminar\r\n\
t=2873397496 2873404696\r\n\
m=video 51372 RTP/AVP 99\r\n\
m=audio 54400 RTP/SAVPF 0 96\r\n\
c=IN IP4 203.0.113.1\r\n\
\r\n";
    #[allow(dead_code)]
    const MEDIA_DESCRIPTION_OUT_OF_ORDER_SDP: &str = "v=0\r\n\
o=jdoe 2890844526 2890842807 IN IP4 10.47.16.5\r\n\
s=SDP Seminar\r\n\
t=2873397496 2873404696\r\n\
m=video 51372 RTP/AVP 99\r\n\
m=audio 54400 RTP/SAVPF 0 96\r\n\
a=rtpmap:99 h263-1998/90000\r\n\
a=candidate:0 1 UDP 2113667327 203.0.113.1 54400 typ host\r\n\
c=IN IP4 203.0.113.1\r\n\
i=Vivamus a posuere nisl\r\n";
    #[allow(dead_code)]
    const MEDIA_DESCRIPTION_OUT_OF_ORDER_SDPACTUAL: &str = "v=0\r\n\
o=jdoe 2890844526 2890842807 IN IP4 10.47.16.5\r\n\
s=SDP Seminar\r\n\
t=2873397496 2873404696\r\n\
m=video 51372 RTP/AVP 99\r\n\
m=audio 54400 RTP/SAVPF 0 96\r\n\
i=Vivamus a posuere nisl\r\n\
c=IN IP4 203.0.113.1\r\n\
a=rtpmap:99 h263-1998/90000\r\n\
a=candidate:0 1 UDP 2113667327 203.0.113.1 54400 typ host\r\n";

    const MEDIA_BANDWIDTH_SDP: &str = "v=0\r\n\
o=jdoe 2890844526 2890842807 IN IP4 10.47.16.5\r\n\
s=SDP Seminar\r\n\
t=2873397496 2873404696\r\n\
m=video 51372 RTP/AVP 99\r\n\
m=audio 54400 RTP/SAVPF 0 96\r\n\
b=X-YZ:128\r\n\
b=AS:12345\r\n";

    const MEDIA_TRANSPORT_BANDWIDTH_SDP: &str = "v=0\r\n\
o=jdoe 2890844526 2890842807 IN IP4 10.47.16.5\r\n\
s=SDP Seminar\r\n\
t=2873397496 2873404696\r\n\
m=video 51372 RTP/AVP 99\r\n\
m=audio 54400 RTP/SAVPF 0 96\r\n\
b=AS:12345\r\n\
b=TIAS:12345\r\n";

    const MEDIA_ENCRYPTION_KEY_SDP: &str = "v=0\r\n\
o=jdoe 2890844526 2890842807 IN IP4 10.47.16.5\r\n\
s=SDP Seminar\r\n\
t=2873397496 2873404696\r\n\
m=video 51372 RTP/AVP 99\r\n\
m=audio 54400 RTP/SAVPF 0 96\r\n\
k=prompt\r\n";

    const MEDIA_ENCRYPTION_KEY_SDPEXTRA_CRLF: &str = "v=0\r\n\
o=jdoe 2890844526 2890842807 IN IP4 10.47.16.5\r\n\
s=SDP Seminar\r\n\
t=2873397496 2873404696\r\n\
m=video 51372 RTP/AVP 99\r\n\
m=audio 54400 RTP/SAVPF 0 96\r\n\
k=prompt\r\n\
\r\n";

    const MEDIA_ATTRIBUTES_SDP: &str = "v=0\r\n\
o=jdoe 2890844526 2890842807 IN IP4 10.47.16.5\r\n\
s=SDP Seminar\r\n\
t=2873397496 2873404696\r\n\
m=video 51372 RTP/AVP 99\r\n\
m=audio 54400 RTP/SAVPF 0 96\r\n\
a=rtpmap:99 h263-1998/90000\r\n\
a=candidate:0 1 UDP 2113667327 203.0.113.1 54400 typ host\r\n\
a=rtcp-fb:97 ccm fir\r\n\
a=rtcp-fb:97 nack\r\n\
a=rtcp-fb:97 nack pli\r\n";

    const CANONICAL_UNMARSHAL_SDP: &str = "v=0\r\n\
o=jdoe 2890844526 2890842807 IN IP4 10.47.16.5\r\n\
s=SDP Seminar\r\n\
i=A Seminar on the session description protocol\r\n\
u=http://www.example.com/seminars/sdp.pdf\r\n\
e=j.doe@example.com (Jane Doe)\r\n\
p=+1 617 555-6011\r\n\
c=IN IP4 224.2.17.12/127\r\n\
b=X-YZ:128\r\n\
b=AS:12345\r\n\
t=2873397496 2873404696\r\n\
t=3034423619 3042462419\r\n\
r=604800 3600 0 90000\r\n\
z=2882844526 -3600 2898848070 0\r\n\
k=prompt\r\n\
a=candidate:0 1 UDP 2113667327 203.0.113.1 54400 typ host\r\n\
a=recvonly\r\n\
m=audio 49170 RTP/AVP 0\r\n\
i=Vivamus a posuere nisl\r\n\
c=IN IP4 203.0.113.1\r\n\
b=X-YZ:128\r\n\
k=prompt\r\n\
a=sendrecv\r\n\
m=video 51372 RTP/AVP 99\r\n\
a=rtpmap:99 h263-1998/90000\r\n";

    #[test]
    fn test_round_trip() -> SDPResult<()> {
        let tests = vec![
            (
                "SessionInformationSDPLFOnly",
                SESSION_INFORMATION_SDPLFONLY,
                Some(SESSION_INFORMATION_SDP),
            ),
            (
                "SessionInformationSDPExtraCRLF",
                SESSION_INFORMATION_SDPEXTRA_CRLF,
                Some(SESSION_INFORMATION_SDP),
            ),
            ("SessionInformation", SESSION_INFORMATION_SDP, None),
            ("URI", URI_SDP, None),
            ("EmailAddress", EMAIL_ADDRESS_SDP, None),
            ("PhoneNumber", PHONE_NUMBER_SDP, None),
            (
                "RepeatTimesSDPExtraCRLF",
                REPEAT_TIMES_SDPEXTRA_CRLF,
                Some(REPEAT_TIMES_SDPEXPECTED),
            ),
            (
                "SessionConnectionInformation",
                SESSION_CONNECTION_INFORMATION_SDP,
                None,
            ),
            ("SessionBandwidth", SESSION_BANDWIDTH_SDP, None),
            ("SessionEncryptionKey", SESSION_ENCRYPTION_KEY_SDP, None),
            (
                "SessionEncryptionKeyExtraCRLF",
                SESSION_ENCRYPTION_KEY_SDPEXTRA_CRLF,
                Some(SESSION_ENCRYPTION_KEY_SDP),
            ),
            ("SessionAttributes", SESSION_ATTRIBUTES_SDP, None),
            //? no repeat no zone
            // (
            //     "TimeZonesSDP2ExtraCRLF",
            //     TIME_ZONES_SDP2EXTRA_CRLF,
            //     Some(TIME_ZONES_SDP2),
            // ),
            ("MediaName", MEDIA_NAME_SDP, None),
            (
                "MediaNameExtraCRLF",
                MEDIA_NAME_SDPEXTRA_CRLF,
                Some(MEDIA_NAME_SDP),
            ),
            ("MediaTitle", MEDIA_TITLE_SDP, None),
            (
                "MediaConnectionInformation",
                MEDIA_CONNECTION_INFORMATION_SDP,
                None,
            ),
            (
                "MediaConnectionInformationExtraCRLF",
                MEDIA_CONNECTION_INFORMATION_SDPEXTRA_CRLF,
                Some(MEDIA_CONNECTION_INFORMATION_SDP),
            ),
            //@ QUOTE: Some lines in each description are required and some are optional, but when present, they must appear in exactly the order given here.
            //@ sdp lines shoud be in order
            // (
            //     "MediaDescriptionOutOfOrder",
            //     MEDIA_DESCRIPTION_OUT_OF_ORDER_SDP,
            //     Some(MEDIA_DESCRIPTION_OUT_OF_ORDER_SDPACTUAL),
            // ),
            ("MediaBandwidth", MEDIA_BANDWIDTH_SDP, None),
            (
                "MediaTransportBandwidth",
                MEDIA_TRANSPORT_BANDWIDTH_SDP,
                None,
            ),
            ("MediaEncryptionKey", MEDIA_ENCRYPTION_KEY_SDP, None),
            (
                "MediaEncryptionKeyExtraCRLF",
                MEDIA_ENCRYPTION_KEY_SDPEXTRA_CRLF,
                Some(MEDIA_ENCRYPTION_KEY_SDP),
            ),
            ("MediaAttributes", MEDIA_ATTRIBUTES_SDP, None),
            ("CanonicalUnmarshal", CANONICAL_UNMARSHAL_SDP, None),
        ];

        for (name, sdp_str, expected) in tests {
            let sdp = SessionDescriptionReader::new().read_from(sdp_str);

            if let Ok(sdp) = sdp {
                let actual = format!("{}", sdp);
                if let Some(expected) = expected {
                    assert_eq!(actual.as_str(), expected, "{name}\n{sdp_str}");
                } else {
                    assert_eq!(actual.as_str(), sdp_str, "{name}\n{sdp_str}");
                }
            } else {
                println!("{:?}", sdp);
                panic!("{name}\n{sdp_str}");
            }
        }

        Ok(())
    }

    #[test]
    fn test_unmarshal_repeat_times() -> SDPResult<()> {
        let sdp = SessionDescriptionReader::new().read_from(REPEAT_TIMES_SDP)?;
        let actual = format!("{}", sdp);
        assert_eq!(actual.as_str(), REPEAT_TIMES_SDPEXPECTED);
        Ok(())
    }

    #[test]
    fn test_unmarshal_repeat_times_overflow() -> SDPResult<()> {
        let result = SessionDescriptionReader::new().read_from(REPEAT_TIMES_OVERFLOW_SDP);
        if let Err(err) = result {
            println!("{:?}", err);
        } else {
            panic!();
        }
        Ok(())
    }

    #[test]
    fn test_unmarshal_time_zones() -> SDPResult<()> {
        let sdp = SessionDescriptionReader::new().read_from(TIME_ZONES_SDP)?;
        let actual = format!("{}", sdp);
        assert_eq!(actual.as_str(), TIME_ZONES_SDPEXPECTED);
        Ok(())
    }
}
