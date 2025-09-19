pub(crate) fn rtp_need_padding(size: usize) -> bool {
    !size.is_multiple_of(4)
}

pub(crate) fn rtp_get_padding_size(size: usize) -> usize {
    (4 - (size % 4)) % 4
}

pub(crate) fn rtp_make_padding_bytes(size: usize) -> Option<Vec<u8>> {
    if !rtp_need_padding(size) {
        return None;
    }

    let padding_size = rtp_get_padding_size(size);
    let mut bytes = vec![0; padding_size];
    bytes[padding_size - 1] = padding_size as u8;
    Some(bytes)
}
