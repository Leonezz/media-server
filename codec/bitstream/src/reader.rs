use std::io;

use bitstream_io::BitRead;
use num::ToPrimitive;

pub struct BitstreamReader<'a> {
    reader: bitstream_io::BitReader<std::io::Cursor<&'a [u8]>, bitstream_io::BigEndian>,
    buf_length: usize,
}

impl<'a> BitstreamReader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        let reader = bitstream_io::BitReader::endian(io::Cursor::new(buf), bitstream_io::BigEndian);
        Self {
            reader,
            buf_length: buf.len(),
        }
    }

    pub fn reader(
        &self,
    ) -> &bitstream_io::BitReader<std::io::Cursor<&'a [u8]>, bitstream_io::BigEndian> {
        &self.reader
    }

    pub fn reader_mut(
        &mut self,
    ) -> &mut bitstream_io::BitReader<std::io::Cursor<&'a [u8]>, bitstream_io::BigEndian> {
        &mut self.reader
    }

    pub fn remaining_bits(&mut self) -> usize {
        self.buf_length
            .checked_mul(8)
            .and_then(|v| {
                v.checked_sub(
                    self.reader
                        .position_in_bits()
                        .map(|v| v.to_usize().unwrap())
                        .unwrap(),
                )
            })
            .unwrap()
    }
}

impl<'a> BitRead for BitstreamReader<'a> {
    fn by_ref(&mut self) -> &mut Self {
        self
    }

    fn byte_align(&mut self) {
        self.reader.byte_align();
    }

    fn byte_aligned(&self) -> bool {
        self.reader.byte_aligned()
    }

    fn parse<F: bitstream_io::FromBitStream>(&mut self) -> Result<F, F::Error> {
        self.reader.parse()
    }

    fn parse_with<'b, F: bitstream_io::FromBitStreamWith<'b>>(
        &mut self,
        context: &F::Context,
    ) -> Result<F, F::Error> {
        self.reader.parse_with(context)
    }

    fn read<const BITS: u32, I>(&mut self) -> io::Result<I>
    where
        I: bitstream_io::Integer,
    {
        self.reader.read::<BITS, I>()
    }

    fn read_as_to<F, V>(&mut self) -> io::Result<V>
    where
        F: bitstream_io::Endianness,
        V: bitstream_io::Primitive,
    {
        self.reader.read_as_to::<F, V>()
    }

    fn read_bit(&mut self) -> io::Result<bool> {
        self.reader.read_bit()
    }

    fn read_bytes(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.reader.read_bytes(buf)
    }

    fn read_const<const BITS: u32, const VALUE: u32, E>(&mut self, err: E) -> Result<(), E>
    where
        E: From<io::Error>,
    {
        self.reader.read_const::<BITS, VALUE, E>(err)
    }

    fn read_count<const MAX: u32>(&mut self) -> io::Result<bitstream_io::BitCount<MAX>> {
        self.reader.read_count::<MAX>()
    }

    fn read_counted<const MAX: u32, I>(
        &mut self,
        bits: bitstream_io::BitCount<MAX>,
    ) -> io::Result<I>
    where
        I: bitstream_io::Integer + Sized,
    {
        self.reader.read_counted::<MAX, I>(bits)
    }

    fn read_huffman<T>(&mut self) -> io::Result<T::Symbol>
    where
        T: bitstream_io::huffman::FromBits,
    {
        self.reader.read_huffman::<T>()
    }

    fn read_signed<const BITS: u32, S>(&mut self) -> io::Result<S>
    where
        S: bitstream_io::SignedInteger,
    {
        self.reader.read_signed::<BITS, S>()
    }

    fn read_signed_counted<const MAX: u32, S>(
        &mut self,
        bits: impl TryInto<bitstream_io::SignedBitCount<MAX>>,
    ) -> io::Result<S>
    where
        S: bitstream_io::SignedInteger,
    {
        self.reader
            .read_signed_counted::<MAX, S>(bits.try_into().map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "signed reads need at least 1 bit for sign",
                )
            })?)
    }

    fn read_signed_var<S>(&mut self, bits: u32) -> io::Result<S>
    where
        S: bitstream_io::SignedInteger,
    {
        self.reader.read_signed_var::<S>(bits)
    }

    fn read_to<V>(&mut self) -> io::Result<V>
    where
        V: bitstream_io::Primitive,
    {
        self.reader.read_to::<V>()
    }

    fn read_to_bytes<const SIZE: usize>(&mut self) -> io::Result<[u8; SIZE]> {
        self.reader.read_to()
    }

    fn read_to_vec(&mut self, bytes: usize) -> io::Result<Vec<u8>> {
        self.reader.read_to_vec(bytes)
    }

    fn read_unary<const STOP_BIT: u8>(&mut self) -> io::Result<u32> {
        self.reader.read_unary::<STOP_BIT>()
    }

    fn read_unsigned<const BITS: u32, U>(&mut self) -> io::Result<U>
    where
        U: bitstream_io::UnsignedInteger,
    {
        self.reader.read_unsigned::<BITS, U>()
    }

    fn read_unsigned_counted<const MAX: u32, U>(
        &mut self,
        bits: bitstream_io::BitCount<MAX>,
    ) -> io::Result<U>
    where
        U: bitstream_io::UnsignedInteger,
    {
        self.reader.read_unsigned_counted::<MAX, U>(bits)
    }

    fn read_unsigned_var<U>(&mut self, bits: u32) -> io::Result<U>
    where
        U: bitstream_io::UnsignedInteger,
    {
        self.reader.read_unsigned_var::<U>(bits)
    }

    fn read_var<I>(&mut self, bits: u32) -> io::Result<I>
    where
        I: bitstream_io::Integer + Sized,
    {
        self.reader.read_var::<I>(bits)
    }

    fn skip(&mut self, bits: u32) -> io::Result<()> {
        self.reader.skip(bits)
    }
}
