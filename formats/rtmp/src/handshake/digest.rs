use hmac::{Hmac, Mac};
use sha2::Sha256;
use tracing::debug;

use super::{
    consts::{RTMP_CLIENT_KEY, RTMP_HANDSHAKE_SIZE, SHA256_DIGEST_SIZE},
    errors::DigestError,
};

// @see: https://blog.csdn.net/win_lin/article/details/13006803
// @see: https://github.com/harlanc/xiu/blob/master/protocol/rtmp/src/handshake/handshake_client.rs

/// two types of schema for c1s1 random bytes:
/// schema1:
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// | key (764 bytes) | digest (764 bytes)  |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///
/// schema2:
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// | digest (764 bytes) | key (764 bytes)  |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///
/// where key:
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// | {offset} bytes  | public key (128 bytes)  | {764 - offset - 128 - 4} bytes  | offset (4bytes) |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// digest:
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// | offset (4 bytes)  | {offset} bytes  | hash digest (32 bytes)  | {764 - 4 - offset - 32} bytes |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
#[derive(Debug)]
enum DigestSchema {
    Schema1,
    Schema2,
}

type DigestResult<T> = Result<T, DigestError>;

fn get_digest_index(random_bytes: &[u8; RTMP_HANDSHAKE_SIZE], schema: DigestSchema) -> usize {
    let mut index: usize = 0;
    match schema {
        DigestSchema::Schema1 => {
            index += random_bytes[772] as usize;
            index += random_bytes[773] as usize;
            index += random_bytes[774] as usize;
            index += random_bytes[775] as usize;
            index %= 728;
            index += 776;
        }
        DigestSchema::Schema2 => {
            index += random_bytes[8] as usize;
            index += random_bytes[9] as usize;
            index += random_bytes[10] as usize;
            index += random_bytes[11] as usize;
            index %= 728;
            index += 12;
        }
    }
    index
}

fn validate_c1_digest_with_schema(
    random_bytes: &[u8; RTMP_HANDSHAKE_SIZE],
    schema: DigestSchema,
) -> DigestResult<Vec<u8>> {
    let index = get_digest_index(random_bytes, schema);
    let left = &random_bytes[..index];
    let hash_digest = &random_bytes[index..index + SHA256_DIGEST_SIZE];
    let right = &random_bytes[index + SHA256_DIGEST_SIZE..];
    let raw_message = [left, right].concat();
    let digest = make_digest(&RTMP_CLIENT_KEY, &raw_message)?;
    if &*digest == hash_digest {
        return Ok(digest);
    }
    debug!(
        "recived digest: {:?}, expected digest: {:?}, split at: {}",
        hash_digest, digest, index,
    );

    Err(DigestError::Invalid)
}

pub fn validate_c1_digest(random_bytes: &[u8; RTMP_HANDSHAKE_SIZE]) -> DigestResult<Vec<u8>> {
    validate_c1_digest_with_schema(random_bytes, DigestSchema::Schema1)
        .or_else(|_| validate_c1_digest_with_schema(random_bytes, DigestSchema::Schema2))
}

pub fn make_digest(key: &[u8], message: &[u8]) -> DigestResult<Vec<u8>> {
    let mut hmac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC can take key of any size");
    hmac.update(message);
    let result = hmac.finalize().into_bytes();
    if result.len() != SHA256_DIGEST_SIZE {
        return Err(DigestError::WrongLength {
            length: result.len(),
        });
    }

    Ok(Vec::from(result.as_slice()))
}

pub fn make_message(key: &[u8], bytes: &[u8; RTMP_HANDSHAKE_SIZE]) -> DigestResult<Vec<u8>> {
    let index = get_digest_index(bytes, DigestSchema::Schema1);
    let left_part = &bytes[..index];
    let right_part = &bytes[index + SHA256_DIGEST_SIZE..];
    let digest = make_c1s1_digest(key, left_part, right_part)?;
    Ok([left_part, digest.as_slice(), right_part].concat())
}

pub fn make_c1s1_digest(key: &[u8], left_part: &[u8], right_part: &[u8]) -> DigestResult<Vec<u8>> {
    let message = [left_part, right_part].concat();
    make_digest(key, &message)
}
