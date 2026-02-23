use tokio_util::{bytes::BytesMut, codec::BytesCodec};

pub trait ProtocolMessage {
    type Codec: tokio_util::codec::Decoder<Item = Self::In, Error = Self::Error>
        + tokio_util::codec::Encoder<Self::Out, Error = Self::Error>
        + Send;
    type In: Send + std::fmt::Debug;
    type Out: Send + std::fmt::Debug;
    type Error: From<std::io::Error>;
    fn codec() -> Self::Codec;
}

impl ProtocolMessage for BytesMut {
    type Codec = BytesCodec;
    type Error = std::io::Error;
    type In = BytesMut;
    type Out = BytesMut;
    fn codec() -> Self::Codec {
        BytesCodec::new()
    }
}
