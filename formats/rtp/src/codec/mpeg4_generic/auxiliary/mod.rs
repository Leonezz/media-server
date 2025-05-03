use tokio_util::bytes::Bytes;

pub mod packet_size;
pub mod read;
pub mod write;

/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+- .. -+-+-+-+-+-+-+-+-+
/// | auxiliary-data-size |   auxiliary-data   | . padding bits  |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+- .. -+-+-+-+-+-+-+-+-+
#[derive(Debug)]
pub struct AuxiliaryData {
    pub auxiliary_data_size: u64,
    pub data: Bytes,
}
