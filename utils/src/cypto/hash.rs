pub fn new_md5_hash(bytes: &[u8]) -> Vec<u8> {
    md5::compute(bytes).0.into()
}

pub fn new_sha256_hash(bytes: &[u8]) -> Vec<u8> {
    ring::digest::digest(&ring::digest::SHA256, bytes)
        .as_ref()
        .into()
}
