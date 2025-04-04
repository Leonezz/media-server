#[cfg(test)]
mod tests {
    use std::io::Read;

    use utils::traits::reader::{ReadFrom, TryReadFrom};

    use crate::{
        consts::{status::RtspStatus, version::RtspVersion},
        header::RtspHeader,
        response::RtspResponse,
    };

    #[test]
    fn options() {
        let response = RtspResponse::builder()
            .version(crate::consts::version::RtspVersion::V2)
            .status(crate::consts::status::RtspStatus::OK)
            .header(RtspHeader::CSeq, "1")
            .header(
                RtspHeader::Public,
                "DESCRIBE, SETUP, TEARDOWN, PLAY, PAUSE, OPTIONS",
            )
            .header(
                RtspHeader::Supported,
                "play.basic, setup.rtp.rtcp.mux, play.scale",
            )
            .header(
                RtspHeader::Server,
                "PhonyServer/1.1",
            )
            .build();
        assert!(response.is_ok());
        let text = "RTSP/2.0 200 OK\r\n\
CSeq: 1\r\n\
Public: DESCRIBE, SETUP, TEARDOWN, PLAY, PAUSE, OPTIONS\r\n\
Supported: play.basic, setup.rtp.rtcp.mux, play.scale\r\n\
Server: PhonyServer/1.1\r\n\r\n";
        assert_eq!(text.trim_end(), format!("{}", response.unwrap()).trim_end());
        let parsed = RtspResponse::read_from(text.as_bytes());
        assert!(parsed.is_ok());
        assert_eq!(text.trim_end(), format!("{}", parsed.unwrap()).trim_end());
    }

    #[test]
    fn describe() {
        let body = "v=0\r\n\
o=MNobody 2890844526 2890842807 IN IP4 192.0.2.46\r\n\
s=SDP Seminar\r\n\
i=A Seminar on the session description protocol\r\n\
u=http://www.example.com/lectures/sdp.ps\r\n\
e=seminar@example.com (Seminar Management)\r\n\
c=IN IP4 0.0.0.0\r\n\
a=control:*\r\n\
t=2873397496 2873404696\r\n\
m=audio 3456 RTP/AVP 0\r\n\
a=control:audio\r\n\
m=video 2232 RTP/AVP 31\r\n\
a=control:video";

        let response = RtspResponse::builder()
            .version(crate::consts::version::RtspVersion::V2)
            .status(crate::consts::status::RtspStatus::OK)
            .header(RtspHeader::CSeq, "312")
            .header(
                RtspHeader::Date,
                "Thu, 23 Jan 1997 15:35:06 GMT",
            )
            .header(
                RtspHeader::Server,
                "PhonyServer/1.1",
            )
            .header(
                RtspHeader::ContentBase,
                "rtsp://server.example.com/fizzle/foo/",
            )
            .header(
                RtspHeader::ContentType,
                "application/sdp",
            )
            .body(body.to_owned())
            .build();

        assert!(response.is_ok());
        let response = response.unwrap();

        let text = format!(
            "{}\r\n{}",
            "RTSP/2.0 200 OK\r\n\
CSeq: 312\r\n\
Date: Thu, 23 Jan 1997 15:35:06 GMT\r\n\
Server: PhonyServer/1.1\r\n\
Content-Base: rtsp://server.example.com/fizzle/foo/\r\n\
Content-Type: application/sdp\r\n\
Content-Length: 343\r\n",
            body
        );
        assert_eq!(text.trim_end(), format!("{}", response).trim_end());
        assert_eq!(response.body().clone().unwrap(), body);

        let parsed = RtspResponse::read_from(text.as_bytes());
        assert!(parsed.is_ok());
        let parsed = parsed.unwrap();
        assert_eq!(text.trim_end(), format!("{}", parsed).trim_end());
        assert!(parsed.body().is_some());
        assert_eq!(parsed.body().clone().unwrap().trim_end(), body);
    }

    #[test]
    fn setup() {
        let response = RtspResponse::builder()
        .version(RtspVersion::V2)
        .status(RtspStatus::OK)
        .header(RtspHeader::CSeq, "302")
        .header(RtspHeader::Date, "Thu, 23 Jan 1997 15:35:06 GMT")
        .header(RtspHeader::Server, "PhonyServer/1.1")
        .header(RtspHeader::Session, "QKyjN8nt2WqbWw4tIYof52;timeout=60")
        .header(RtspHeader::Transport, "RTP/AVP;unicast;dest_addr=\"192.0.2.53:4588\"/\"192.0.2.53:4589\"; src_addr=\"198.51.100.241:6256\"/\"198.51.100.241:6257\"; ssrc=2A3F93ED")
        .header(RtspHeader::AcceptRanges, "npt")
        .header(RtspHeader::MediaProperties, "Random-Access=3.2, Time-Progressing, Time-Duration=3600.0")
        .header(RtspHeader::MediaRange, "npt=0-2893.23")
        .build();
        assert!(response.is_ok());
        let response = response.unwrap();
        let text = "RTSP/2.0 200 OK\r\n\
CSeq: 302\r\n\
Date: Thu, 23 Jan 1997 15:35:06 GMT\r\n\
Server: PhonyServer/1.1\r\n\
Session: QKyjN8nt2WqbWw4tIYof52;timeout=60\r\n\
Transport: RTP/AVP;unicast;dest_addr=\"192.0.2.53:4588\"/\"192.0.2.53:4589\"; src_addr=\"198.51.100.241:6256\"/\"198.51.100.241:6257\"; ssrc=2A3F93ED\r\n\
Accept-Ranges: npt\r\n\
Media-Properties: Random-Access=3.2, Time-Progressing, Time-Duration=3600.0\r\n\
Media-Range: npt=0-2893.23\r\n\r\n";
        assert_eq!(text.trim_end(), format!("{}", response).trim_end());
        let parsed = RtspResponse::read_from(text.as_bytes());
        assert!(parsed.is_ok());
        assert_eq!(text.trim_end(), format!("{}", parsed.unwrap()).trim_end());
    }

    #[test]
    fn play() {
        let response = RtspResponse::builder()
            .version(RtspVersion::V2)
            .status(RtspStatus::OK)
            .header(RtspHeader::CSeq, "836")
            .header(RtspHeader::Date, "Thu, 23 Jan 1997 15:35:06 GMT")
            .header(RtspHeader::Server, "PhonyServer/1.0")
            .header(RtspHeader::Range, "npt=3.51-324.39")
            .header(RtspHeader::SeekStyle, "First-Prior")
            .header(RtspHeader::Session, "ULExwZCXh2pd0xuFgkgZJW")
            .header(
                RtspHeader::RtpInfo,
                "url=\"rtsp://example.com/audio\" ssrc=0D12F123:seq=14783;rtptime=2345962545",
            )
            .build();
        assert!(response.is_ok());
        let response = response.unwrap();
        let text = "RTSP/2.0 200 OK\r\n\
CSeq: 836\r\n\
Date: Thu, 23 Jan 1997 15:35:06 GMT\r\n\
Server: PhonyServer/1.0\r\n\
Range: npt=3.51-324.39\r\n\
Seek-Style: First-Prior\r\n\
Session: ULExwZCXh2pd0xuFgkgZJW\r\n\
RTP-Info: url=\"rtsp://example.com/audio\" ssrc=0D12F123:seq=14783;rtptime=2345962545\r\n\r\n";
        assert_eq!(text.trim_end(), format!("{}", response).trim_end());
        let parsed = RtspResponse::read_from(text.as_bytes());
        assert!(parsed.is_ok());
        assert_eq!(text.trim_end(), format!("{}", parsed.unwrap()).trim_end());
    }

    #[test]
    fn play_notify() {
        let response = RtspResponse::builder()
            .version(RtspVersion::V2)
            .status(RtspStatus::OK)
            .header(RtspHeader::CSeq, "854")
            .header(RtspHeader::UserAgent, "PhonyClient/1.2")
            .header(RtspHeader::Session, "CDtUJfDQXJWtJ7Iqua2xOi")
            .build();
        assert!(response.is_ok());
        let text = "RTSP/2.0 200 OK\r\n\
CSeq: 854\r\n\
User-Agent: PhonyClient/1.2\r\n\
Session: CDtUJfDQXJWtJ7Iqua2xOi\r\n\r\n";
        let response = response.unwrap();
        assert_eq!(text.trim_end(), format!("{}", response).trim_end());
        let parsed = RtspResponse::read_from(text.as_bytes());
        assert!(parsed.is_ok());
        assert_eq!(text.trim_end(), format!("{}", parsed.unwrap()).trim_end());
    }

    #[test]
    fn pause() {
        let response = RtspResponse::builder()
            .version(RtspVersion::V2)
            .status(RtspStatus::OK)
            .header(RtspHeader::CSeq, "834")
            .header(RtspHeader::Date, "Thu, 23 Jan 1997 15:35:06 GMT")
            .header(RtspHeader::Session, "OoOUPyUwt0VeY9fFRHuZ6L")
            .header(RtspHeader::Range, "npt=45.76-75.00")
            .build();
        assert!(response.is_ok());
        let text = "RTSP/2.0 200 OK\r\n\
CSeq: 834\r\n\
Date: Thu, 23 Jan 1997 15:35:06 GMT\r\n\
Session: OoOUPyUwt0VeY9fFRHuZ6L\r\n\
Range: npt=45.76-75.00\r\n\r\n";
        let response = response.unwrap();
        assert_eq!(text.trim_end(), format!("{}", response).trim_end());
        let parsed = RtspResponse::read_from(text.as_bytes());
        assert!(parsed.is_ok());
        assert_eq!(text.trim_end(), format!("{}", parsed.unwrap()).trim_end());
    }

    #[test]
    fn teardown() {
        let response = RtspResponse::builder()
            .version(RtspVersion::V2)
            .status(RtspStatus::OK)
            .header(RtspHeader::CSeq, "892")
            .header(RtspHeader::Server, "PhonyServer/1.0")
            .build();
        assert!(response.is_ok());
        let text = "RTSP/2.0 200 OK\r\n\
CSeq: 892\r\n\
Server: PhonyServer/1.0\r\n\r\n";
        let response = response.unwrap();
        assert_eq!(text.trim_end(), format!("{}", response).trim_end());
        let parsed = RtspResponse::read_from(text.as_bytes());
        assert!(parsed.is_ok());
        assert_eq!(text.trim_end(), format!("{}", parsed.unwrap()).trim_end());
    }

    #[test]
    fn get_parameter() {
        let response = RtspResponse::builder()
            .version(RtspVersion::V2)
            .status(RtspStatus::OK)
            .header(RtspHeader::CSeq, "431")
            .header(RtspHeader::Session, "OccldOFFq23KwjYpAnBbUr")
            .header(RtspHeader::Server, "PhonyServer/1.1")
            .header(RtspHeader::Date, "Mon, 08 Mar 2010 13:43:23 GMT")
            .header(RtspHeader::ContentLength, "36")
            .header(RtspHeader::ContentType, "text/parameters")
            .body("packets_received: 10\r\njitter: 0.3838".to_owned())
            .build();
        assert!(response.is_ok());
        let text = "RTSP/2.0 200 OK\r\n\
CSeq: 431\r\n\
Session: OccldOFFq23KwjYpAnBbUr\r\n\
Server: PhonyServer/1.1\r\n\
Date: Mon, 08 Mar 2010 13:43:23 GMT\r\n\
Content-Type: text/parameters\r\n\
Content-Length: 36\r\n";
        let body = "packets_received: 10\r\njitter: 0.3838";
        let text = format!("{}\r\n{}", text, body);
        let response = response.unwrap();
        assert_eq!(text.trim_end(), format!("{}", response).trim_end());
        assert!(response.body().is_some());
        assert_eq!(response.body().clone().unwrap(), body);

        let parsed = RtspResponse::read_from(text.as_bytes());
        assert!(parsed.is_ok());
        assert_eq!(text.trim_end(), format!("{}", parsed.unwrap()).trim_end());
    }

    #[test]
    fn get_parameter_incomplete() {
        let text = "RTSP/2.0 200 OK\r\n\
CSeq: 431\r\n\
Session: OccldOFFq23KwjYpAnBbUr\r\n\
Server: PhonyServer/1.1\r\n\
Date: Mon, 08 Mar 2010 13:43:23 GMT\r\n\
Content-Type: text/parameters\r\n\
Content-Length: 36\r\n";
        let body = "packets_received: 10\r\njitter: ";
        let text = format!("{}\r\n{}", text, body);
        let mut cursor = std::io::Cursor::new(text.as_bytes());
        let response = RtspResponse::try_read_from(cursor.by_ref());
        assert!(response.is_ok());
        let response = response.unwrap();
        assert!(response.is_none());
    }

    #[test]
    fn set_parameter() {
        let response = RtspResponse::builder()
            .version(RtspVersion::V2)
            .status(RtspStatus::ParameterNotUnderstood)
            .header(RtspHeader::CSeq, "421")
            .header(RtspHeader::Session, "iixT43KLc")
            .header(RtspHeader::Server, "PhonyServer/1.0")
            .header(RtspHeader::Date, "Mon, 08 Mar 2010 14:44:56 GMT")
            .header(RtspHeader::ContentType, "text/parameters")
            .body("barparam: barstuff".to_owned())
            .build();
        assert!(response.is_ok());
        let text = "RTSP/2.0 451 Parameter Not Understood\r\n\
CSeq: 421\r\n\
Session: iixT43KLc\r\n\
Server: PhonyServer/1.0\r\n\
Date: Mon, 08 Mar 2010 14:44:56 GMT\r\n\
Content-Type: text/parameters\r\n\
Content-Length: 18\r\n";
        let body = "barparam: barstuff";
        let text = format!("{}\r\n{}", text, body);
        let response = response.unwrap();
        assert_eq!(text.trim_end(), format!("{}", response).trim_end());
        assert!(response.body().is_some());
        assert_eq!(body, response.body().clone().unwrap());

        let parsed = RtspResponse::read_from(text.as_bytes());
        assert!(parsed.is_ok());
        let parsed = parsed.unwrap();
        assert_eq!(text.trim_end(), format!("{}", parsed).trim_end());
        assert!(parsed.body().is_some());
        assert_eq!(body, parsed.body().clone().unwrap());
    }

    #[test]
    fn redirect() {
        let response = RtspResponse::builder()
            .version(RtspVersion::V2)
            .status(RtspStatus::OK)
            .header(RtspHeader::CSeq, "732")
            .header(RtspHeader::UserAgent, "PhonyClient/1.2")
            .header(RtspHeader::Session, "uZ3ci0K+Ld-M")
            .build();
        assert!(response.is_ok());
        let text = "RTSP/2.0 200 OK\r\n\
CSeq: 732\r\n\
User-Agent: PhonyClient/1.2\r\n\
Session: uZ3ci0K+Ld-M\r\n\r\n";
        let response = response.unwrap();
        assert_eq!(text.trim_end(), format!("{}", response).trim_end());
        assert!(response.body().is_none());
        let parsed = RtspResponse::read_from(text.as_bytes());
        assert!(parsed.is_ok());
        let parsed = parsed.unwrap();
        assert_eq!(text.trim_end(), format!("{}", parsed).trim_end());
        assert!(parsed.body().is_none());
    }

    #[test]
    fn redirect_incomplete() {
        let text = "RTSP/2.0 200 OK\r\n\
CSeq: 732\r\n\
User-Agent: PhonyClient/1.2\r\n\
Session: uZ3ci0K+Ld-M";

        let mut cursor = std::io::Cursor::new(text.as_bytes());
        let response = RtspResponse::try_read_from(cursor.by_ref());
        assert!(response.is_ok());
        let response = response.unwrap();
        assert!(response.is_none());
    }
}
