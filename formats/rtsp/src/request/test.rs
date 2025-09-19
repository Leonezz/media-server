#[cfg(test)]
mod tests {
    use std::io::{self, Read};

    use url::Url;
    use utils::traits::reader::{ReadFrom, TryReadFrom};

    use crate::{
        consts::{methods::RtspMethod, version::RtspVersion},
        header::RtspHeader,
        request::RtspRequest,
    };

    #[test]
    fn options() {
        let request = RtspRequest::builder()
            .method(crate::consts::methods::RtspMethod::Options)
            .uri("rtsp://server.example.com".parse::<Url>().unwrap())
            .version(RtspVersion::V2)
            .header(RtspHeader::CSeq, "1")
            .header(RtspHeader::UserAgent, "PhonyClient/1.2")
            .header(RtspHeader::ProxyRequire, "gzipped-messages")
            .header(RtspHeader::Supported, "play.basic")
            .build();
        assert!(request.is_ok());

        let text = "OPTIONS rtsp://server.example.com RTSP/2.0\r\n\
CSeq: 1\r\n\
User-Agent: PhonyClient/1.2\r\n\
Proxy-Require: gzipped-messages\r\n\
Supported: play.basic\r\n\r\n";

        assert_eq!(format!("{}", request.unwrap()).trim_end(), text.trim_end());
        let parsed = RtspRequest::read_from(&mut text.as_bytes());
        assert!(parsed.is_ok());
        assert_eq!(text.trim_end(), format!("{}", parsed.unwrap()).trim_end());
    }

    #[test]
    fn describe() {
        let request = RtspRequest::builder()
            .method(crate::consts::methods::RtspMethod::Describe)
            .uri(
                "rtsp://server.example.com/fizzle/foo"
                    .parse::<Url>()
                    .unwrap(),
            )
            .version(RtspVersion::V2)
            .header(RtspHeader::CSeq, "312")
            .header(RtspHeader::UserAgent, "PhonyClient/1.2")
            .header(RtspHeader::Accept, "application/sdp, application/example")
            .build();
        assert!(request.is_ok());
        let text = "DESCRIBE rtsp://server.example.com/fizzle/foo RTSP/2.0\r\n\
CSeq: 312\r\n\
User-Agent: PhonyClient/1.2\r\n\
Accept: application/sdp, application/example\r\n\r\n";
        assert_eq!(text.trim_end(), format!("{}", request.unwrap()).trim_end());
        let parsed = RtspRequest::read_from(&mut text.as_bytes());
        assert!(parsed.is_ok());
        assert_eq!(text.trim_end(), format!("{}", parsed.unwrap()).trim_end());
    }

    #[test]
    fn setup() {
        let request = RtspRequest::builder()
            .method(crate::consts::methods::RtspMethod::Setup)
            .uri("rtsp://example.com/foo/bar/baz.rm".parse::<Url>().unwrap())
            .version(RtspVersion::V2)
            .header(RtspHeader::CSeq, "302")
            .header(RtspHeader::Transport, "RTP/AVP;unicast;dest_addr=\":4588\"/\":4589\", RTP/AVP/TCP;unicast;interleaved=0-1")
            .header(RtspHeader::AcceptRanges, "npt, clock")
            .header(RtspHeader::UserAgent, "PhonyClient/1.2")
            .build();

        assert!(request.is_ok());
        let request = request.unwrap();

        let text = "SETUP rtsp://example.com/foo/bar/baz.rm RTSP/2.0\r\n\
CSeq: 302\r\n\
Transport: RTP/AVP;unicast;dest_addr=\":4588\"/\":4589\", RTP/AVP/TCP;unicast;interleaved=0-1\r\n\
Accept-Ranges: npt, clock\r\n\
User-Agent: PhonyClient/1.2\r\n\r\n";
        assert_eq!(text.trim_end(), format!("{}", request).trim_end());
        let parsed = RtspRequest::read_from(&mut text.as_bytes());
        assert!(parsed.is_ok());
        assert_eq!(text.trim_end(), format!("{}", parsed.unwrap()).trim_end());
    }

    #[test]
    fn play() {
        let request = RtspRequest::builder()
            .method(RtspMethod::Play)
            .uri("rtsp://example.com/audio".parse::<Url>().unwrap())
            .version(RtspVersion::V2)
            .header(RtspHeader::CSeq, "836")
            .header(RtspHeader::Session, "ULExwZCXh2pd0xuFgkgZJW")
            .header(RtspHeader::Range, "npt=3.52-")
            .header(RtspHeader::UserAgent, "PhonyClient/1.2")
            .build();
        assert!(request.is_ok());
        let request = request.unwrap();
        let text = "PLAY rtsp://example.com/audio RTSP/2.0\r\n\
CSeq: 836\r\n\
Session: ULExwZCXh2pd0xuFgkgZJW\r\n\
Range: npt=3.52-\r\n\
User-Agent: PhonyClient/1.2\r\n\r\n";
        assert_eq!(text.trim_end(), format!("{}", request).trim_end());
        let parsed = RtspRequest::read_from(&mut text.as_bytes());
        assert!(parsed.is_ok());
        assert_eq!(text.trim_end(), format!("{}", parsed.unwrap()).trim_end());
    }

    #[test]
    fn play_notify() {
        let request = RtspRequest::builder()
            .method(RtspMethod::PlayNotify)
            .uri("rtsp://example.com/fizzle/foo".parse::<Url>().unwrap())
            .version(RtspVersion::V2)
            .header(RtspHeader::CSeq, "854")
            .header(RtspHeader::NotifyReason, "end-of-stream")
            .header(RtspHeader::RequestStatus, "cseq=853 status=200 reason=\"OK\"")
            .header(RtspHeader::Range, "npt=-145")
            .header(RtspHeader::RtpInfo, "url=\"rtsp://example.com/fizzle/foo/audio\" ssrc=0D12F123:seq=14783;rtptime=2345962545, url=\"rtsp://example.com/fizzle/video\" ssrc=789DAF12:seq=57654;rtptime=2792482193")
            .header(RtspHeader::Session, "CDtUJfDQXJWtJ7Iqua2xOi")
            .header(RtspHeader::Date, "Mon, 08 Mar 2010 13:37:16 GMT")
            .build();
        assert!(request.is_ok());
        let text = "PLAY_NOTIFY rtsp://example.com/fizzle/foo RTSP/2.0\r\n\
CSeq: 854\r\n\
Notify-Reason: end-of-stream\r\n\
Request-Status: cseq=853 status=200 reason=\"OK\"\r\n\
Range: npt=-145\r\n\
RTP-Info: url=\"rtsp://example.com/fizzle/foo/audio\" ssrc=0D12F123:seq=14783;rtptime=2345962545, url=\"rtsp://example.com/fizzle/video\" ssrc=789DAF12:seq=57654;rtptime=2792482193\r\n\
Session: CDtUJfDQXJWtJ7Iqua2xOi\r\n\
Date: Mon, 08 Mar 2010 13:37:16 GMT\r\n\r\n";
        let request = request.unwrap();
        assert_eq!(text.trim_end(), format!("{}", request).trim_end());
        let parsed = RtspRequest::read_from(&mut text.as_bytes());
        assert!(parsed.is_ok());
        assert_eq!(text.trim_end(), format!("{}", parsed.unwrap()).trim_end());
    }

    #[test]
    fn pause() {
        let request = RtspRequest::builder()
            .method(RtspMethod::Pause)
            .uri("rtsp://example.com/fizzle/foo".parse::<Url>().unwrap())
            .version(RtspVersion::V2)
            .header(RtspHeader::CSeq, "834")
            .header(RtspHeader::Session, "OoOUPyUwt0VeY9fFRHuZ6L")
            .header(RtspHeader::UserAgent, "PhonyClient/1.2")
            .build();
        assert!(request.is_ok());
        let text = "PAUSE rtsp://example.com/fizzle/foo RTSP/2.0\r\n\
CSeq: 834\r\n\
Session: OoOUPyUwt0VeY9fFRHuZ6L\r\n\
User-Agent: PhonyClient/1.2\r\n\r\n";
        let request = request.unwrap();
        assert_eq!(text.trim_end(), format!("{}", request).trim_end());
        let parsed = RtspRequest::read_from(&mut text.as_bytes());
        assert!(parsed.is_ok());
        assert_eq!(text.trim_end(), format!("{}", parsed.unwrap()).trim_end());
    }

    #[test]
    fn teardown() {
        let request = RtspRequest::builder()
            .method(RtspMethod::TearDown)
            .uri("rtsp://example.com/fizzle/foo".parse::<Url>().unwrap())
            .version(RtspVersion::V2)
            .header(RtspHeader::CSeq, "892")
            .header(RtspHeader::Session, "OccldOFFq23KwjYpAnBbUr")
            .header(RtspHeader::UserAgent, "PhonyClient/1.2")
            .build();
        assert!(request.is_ok());
        let request = request.unwrap();
        let text = "TEARDOWN rtsp://example.com/fizzle/foo RTSP/2.0\r\n\
CSeq: 892\r\n\
Session: OccldOFFq23KwjYpAnBbUr\r\n\
User-Agent: PhonyClient/1.2\r\n\r\n";
        assert_eq!(text.trim_end(), format!("{}", request).trim_end());
        let parsed = RtspRequest::read_from(&mut text.as_bytes());
        assert!(parsed.is_ok());
        assert_eq!(text.trim_end(), format!("{}", parsed.unwrap()).trim_end());
    }

    #[test]
    fn get_parameter() {
        let request = RtspRequest::builder()
            .method(RtspMethod::GetParameter)
            .uri("rtsp://example.com/fizzle/foo".parse::<Url>().unwrap())
            .version(RtspVersion::V2)
            .header(RtspHeader::CSeq, "431")
            .header(RtspHeader::UserAgent, "PhonyClient/1.2")
            .header(RtspHeader::Session, "OccldOFFq23KwjYpAnBbUr")
            .header(RtspHeader::ContentType, "text/parameters")
            .header(RtspHeader::ContentLength, "24")
            .body("packets_received\r\njitter".to_owned())
            .build();
        assert!(request.is_ok());
        let request = request.unwrap();
        let text = "GET_PARAMETER rtsp://example.com/fizzle/foo RTSP/2.0\r\n\
CSeq: 431\r\n\
User-Agent: PhonyClient/1.2\r\n\
Session: OccldOFFq23KwjYpAnBbUr\r\n\
Content-Type: text/parameters\r\n\
Content-Length: 24\r\n";

        let body = "packets_received\r\njitter";
        let text = format!("{}\r\n{}", text, body);
        assert_eq!(text.trim_end(), format!("{}", request).trim_end());
        assert!(request.body().is_some());
        assert_eq!(request.body().unwrap(), body);
        let parsed = RtspRequest::read_from(&mut text.as_bytes());
        assert!(parsed.is_ok());
        let parsed = parsed.unwrap();
        assert_eq!(text, format!("{}", parsed));
        assert!(parsed.clone().body().is_some());
        assert_eq!(parsed.clone().body().unwrap(), body);
    }

    #[test]
    fn get_parameter_incomplete() {
        let text = "GET_PARAMETER rtsp://example.com/fizzle/foo RTSP/2.0\r\n\
CSeq: 431\r\n\
User-Agent: PhonyClient/1.2\r\n\
Session: OccldOFFq23KwjYpAnBbUr\r\n\
Content-Type: text/parameters\r\n\
Content-Length: 24\r\n\r\n";
        let body = "packets_received\r\njitt";
        let text = format!("{}\r\n{}", text, body);

        let mut cursor = io::Cursor::new(text.as_bytes());
        let parsed = RtspRequest::try_read_from(cursor.by_ref());
        assert!(parsed.is_ok());
        let parsed = parsed.unwrap();
        assert!(parsed.is_none());
    }

    #[test]
    fn set_parameter() {
        let request = RtspRequest::builder()
            .method(RtspMethod::SetParameter)
            .uri("rtsp://example.com/fizzle/foo".parse::<Url>().unwrap())
            .version(RtspVersion::V2)
            .header(RtspHeader::CSeq, "421")
            .header(RtspHeader::UserAgent, "PhonyClient/1.2")
            .header(RtspHeader::Session, "iixT43KLc")
            .header(RtspHeader::Date, "Mon, 08 Mar 2010 14:45:04 GMT")
            .header(RtspHeader::ContentType, "text/parameters")
            .body("barparam: barstuff".to_owned())
            .build();
        assert!(request.is_ok());
        let request = request.unwrap();
        let text = "SET_PARAMETER rtsp://example.com/fizzle/foo RTSP/2.0\r\n\
CSeq: 421\r\n\
User-Agent: PhonyClient/1.2\r\n\
Session: iixT43KLc\r\n\
Date: Mon, 08 Mar 2010 14:45:04 GMT\r\n\
Content-Type: text/parameters\r\n\
Content-Length: 18\r\n";
        let body = "barparam: barstuff";
        let text = format!("{}\r\n{}", text, body);
        assert_eq!(text.trim_end(), format!("{}", request).trim_end());
        assert!(request.body().is_some());
        assert_eq!(request.body().unwrap(), body);

        let parsed = RtspRequest::read_from(&mut text.as_bytes());
        assert!(parsed.is_ok());
        let parsed = parsed.unwrap();
        assert_eq!(text.trim_end(), format!("{}", parsed));
        assert_eq!(body, parsed.body().unwrap());
    }

    #[test]
    fn redirect() {
        let request = RtspRequest::builder()
            .method(RtspMethod::Redirect)
            .uri("rtsp://example.com/fizzle/foo".parse::<Url>().unwrap())
            .version(RtspVersion::V2)
            .header(RtspHeader::CSeq, "732")
            .header(
                RtspHeader::Location,
                "rtsp://s2.example.com:8001/fizzle/foo",
            )
            .header(
                RtspHeader::TerminateReason,
                "Server-Admin ;time=19960213T143205Z",
            )
            .header(RtspHeader::Session, "uZ3ci0K+Ld-M")
            .header(RtspHeader::Date, "Thu, 13 Feb 1996 14:30:43 GMT")
            .build();
        assert!(request.is_ok());
        let text = "REDIRECT rtsp://example.com/fizzle/foo RTSP/2.0\r\n\
CSeq: 732\r\n\
Location: rtsp://s2.example.com:8001/fizzle/foo\r\n\
Terminate-Reason: Server-Admin ;time=19960213T143205Z\r\n\
Session: uZ3ci0K+Ld-M\r\n\
Date: Thu, 13 Feb 1996 14:30:43 GMT\r\n\r\n";
        let request = request.unwrap();
        assert_eq!(text.trim_end(), format!("{}", request).trim_end());
    }

    #[test]
    fn redirect_incomplete() {
        let text = "REDIRECT rtsp://example.com/fizzle/foo RTSP/2.0\r\n\
CSeq: 732\r\n\
Location: rtsp://s2.example.com:8001/fizzle/foo\r\n\
Terminate-Reason: Server-Admin ;time=19960213T143205Z\r\n\
Session: uZ3ci0K+Ld-M\r\n\
Date: Thu, 13 Feb 1996 14:30:43 GMT";

        let mut cursor = io::Cursor::new(text.as_bytes());
        let parsed = RtspRequest::try_read_from(cursor.by_ref());
        assert!(parsed.is_ok());
        let parsed = parsed.unwrap();
        assert!(parsed.is_none());
    }
}
