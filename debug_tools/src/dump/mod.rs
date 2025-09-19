use std::io;

use bitstream_io::BitWrite;
use utils::traits::writer::{BitwiseWriteTo, WriteTo};
pub mod file_dump;
pub trait DumpTool {
    fn dump(&mut self, data: &impl WriteTo<Vec<u8>>) -> io::Result<()> {
        let mut writer = Vec::new();
        data.write_to(&mut writer)
            .map_err(|e| io::Error::other(format!("{:?}", e)))?;
        self.dump_bytes(&writer)?;
        Ok(())
    }
    fn dump_bits(
        &mut self,
        data: &impl BitwiseWriteTo<bitstream_io::BitWriter<Vec<u8>, bitstream_io::BigEndian>>,
    ) -> io::Result<()> {
        let writer = self.bit_writer();
        data.write_to(writer)
            .map_err(|e| io::Error::other(format!("{:?}", e)))?;
        Ok(())
    }
    fn dump_bytes(&mut self, data: &impl AsRef<[u8]>) -> io::Result<()> {
        let writer = self.bit_writer();
        if !writer.byte_aligned() {
            tracing::warn!("dump bytes but bit writer is not byte aligned, align it first");
            writer
                .byte_align()
                .map_err(|e| io::Error::other(format!("byte align failed: {:?}", e)))?;
        }
        writer
            .write_bytes(data.as_ref())
            .map_err(|e| io::Error::other(format!("dump bytes failed: {:?}", e)))?;
        self.flush()?;
        Ok(())
    }
    fn flush(&mut self) -> io::Result<()>;
    fn get_id(&self) -> &str;
    fn bit_writer(&mut self) -> &mut bitstream_io::BitWriter<Vec<u8>, bitstream_io::BigEndian>;
    fn bytes_dumped(&self) -> usize;
}
