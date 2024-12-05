pub const RTMP_SERVER_KEY_FIRST_HALF: &str = "Genuine Adobe Flash Media Server 001";
pub const RTMP_CLIENT_KEY_FIRST_HALF: &str = "Genuine Adobe Flash Player 001";

pub const RTMP_SERVER_KEY: [u8; 68] = [
    b'G', b'e', b'n', b'u', b'i', b'n', b'e', b' ', b'A', b'd', b'o', b'b', b'e', b'i', b'F', b'l',
    b'a', b's', b'h', b' ', b'M', b'e', b'd', b'i', b'a', b' ', b'S', b'e', b'r', b'v', b'e', b'r',
    b' ', b'0', b'0', b'1', /* Genuine Adobe Flash Media Server 001 */
    0xF0, 0xEE, 0xC2, 0x4A, 0x80, 0x68, 0xBE, 0xE8, 0x2E, 0x00, 0xD0, 0xD1, 0x02, 0x9E, 0x7E, 0x57,
    0x6E, 0xEC, 0x5D, 0x2D, 0x29, 0x80, 0x6F, 0xAB, 0x93, 0xB8, 0xE6, 0x36, 0xCF, 0xEB, 0x31, 0xAE,
];

pub const RTMP_CLIENT_KEY: [u8; 62] = [
    b'G', b'e', b'n', b'u', b'i', b'n', b'e', b' ', b'A', b'd', b'o', b'b', b'e', b' ', b'F', b'l',
    b'a', b's', b'h', b' ', b'P', b'l', b'a', b'y', b'e', b'r', b' ', b'0', b'0',
    b'1', /* Genuine Adobe Flash Player 001 */
    0xF0, 0xEE, 0xC2, 0x4A, 0x80, 0x68, 0xBE, 0xE8, 0x2E, 0x00, 0xD0, 0xD1, 0x02, 0x9E, 0x7E, 0x57,
    0x6E, 0xEC, 0x5D, 0x2D, 0x29, 0x80, 0x6F, 0xAB, 0x93, 0xB8, 0xE6, 0x36, 0xCF, 0xEB, 0x31, 0xAE,
];

pub struct FourBytes([u8; 4]);
pub const RTMP_SERVER_VERSION: FourBytes = FourBytes([0x0D, 0x0E, 0x0A, 0x0D]);
pub const RTMP_CLIENT_VERSION: FourBytes = FourBytes([0x0C, 0x00, 0x0D, 0x0E]);
pub const RTMP_HANDSHAKE_SIZE: usize = 1536;
pub const SHA256_DIGEST_SIZE: usize = 32;

impl Into<u32> for FourBytes {
    fn into(self) -> u32 {
        let mut res: u32 = 0;
        res <<= 8;
        res |= self.0[0] as u32;
        res <<= 8;
        res |= self.0[1] as u32;
        res <<= 8;
        res |= self.0[2] as u32;
        res <<= 8;
        res |= self.0[3] as u32;
        res
    }
}
