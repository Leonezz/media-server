use std::num::NonZero;

pub fn new_hmac(alg: ring::hmac::Algorithm, key: &[u8], message: &[u8]) -> Vec<u8> {
    let mac = ring::hmac::Key::new(alg, key);
    ring::hmac::sign(&mac, message).as_ref().to_vec()
}

pub fn new_sha1_hmac(key: &[u8], message: &[u8]) -> Vec<u8> {
    new_hmac(ring::hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY, key, message)
}

pub fn new_sha256_hmac(key: &[u8], message: &[u8]) -> Vec<u8> {
    new_hmac(ring::hmac::HMAC_SHA256, key, message)
}

pub fn new_pbkdf2_sha256_hmac(
    password: &[u8],
    salt: &[u8],
    iteration: usize,
    out_len: usize,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(out_len);
    ring::pbkdf2::derive(
        ring::pbkdf2::PBKDF2_HMAC_SHA256,
        NonZero::<u32>::new(iteration as u32).unwrap(),
        salt,
        password,
        &mut out,
    );
    out
}
