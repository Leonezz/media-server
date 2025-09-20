use crate::dump::DumpTool;
use std::{
    fmt::Debug,
    io::{self, Write},
};

pub struct FileDump {
    file: std::fs::File,
    writer: bitstream_io::BitWriter<Vec<u8>, bitstream_io::BigEndian>,
    bytes_written: usize,
    name: String,
}

impl Debug for FileDump {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "name: {}, bytes_written: {}",
            self.name, self.bytes_written
        )
    }
}

impl FileDump {
    pub fn new(name: &str) -> std::io::Result<Self> {
        if std::path::Path::new(name).exists() {
            std::fs::remove_file(name)?;
        } else {
            std::fs::create_dir_all(
                std::path::Path::new(name)
                    .parent()
                    .unwrap_or(std::path::Path::new("./")),
            )?;
        }
        let file = std::fs::File::create(name)?;
        let buffer = Vec::with_capacity(1024);
        let writer = bitstream_io::BitWriter::endian(buffer, bitstream_io::BigEndian);
        Ok(Self {
            file,
            name: name.to_string(),
            writer,
            bytes_written: 0,
        })
    }
}

impl DumpTool for FileDump {
    fn bit_writer(&mut self) -> &mut bitstream_io::BitWriter<Vec<u8>, bitstream_io::BigEndian> {
        &mut self.writer
    }

    fn get_id(&self) -> &str {
        &self.name
    }

    fn flush(&mut self) -> io::Result<()> {
        let buffer = self
            .writer
            .writer()
            .expect("bit writer is not byte aligned");
        self.bytes_written += buffer.len();
        self.file.write_all(buffer)?;
        self.file.flush()?;
        self.writer = bitstream_io::BitWriter::endian(Vec::new(), bitstream_io::BigEndian);
        Ok(())
    }

    fn bytes_dumped(&self) -> usize {
        self.bytes_written
    }
}
