pub trait ProtocolMessage {
    type Codec: tokio_util::codec::Decoder<Item = Self::In, Error = Self::Error>
        + tokio_util::codec::Encoder<Self::Out, Error = Self::Error>
        + Send;
    type In: Send + std::fmt::Debug;
    type Out: Send + std::fmt::Debug;
    type Error: From<std::io::Error>;
    fn codec() -> Self::Codec;
}
