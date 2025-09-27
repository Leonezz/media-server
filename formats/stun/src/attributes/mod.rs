use crate::{
    attribute::AttrType,
    errors::{STUNMessageError, STUNMessageResult},
};

pub mod rfc8489;
pub fn check_attr_match(from_attr: AttrType, to_attr: AttrType) -> STUNMessageResult<()> {
    if from_attr != to_attr {
        return Err(STUNMessageError::SyntaxError(format!(
            "attr {:?} and {:?} not match",
            from_attr, to_attr
        )));
    }
    Ok(())
}

pub const STUN_ATTRIBUTE_PADDING_SIZE: usize = 4;

pub fn need_padding(len: usize, padding_to: usize) -> bool {
    !len.is_multiple_of(padding_to)
}

pub fn get_after_padding_size(len: usize, padding_to: usize) -> usize {
    assert_ne!(padding_to, 0);
    let size = padding_to * (len / padding_to);
    if size < len {
        return size + padding_to;
    }
    size
}

pub fn make_padding(buf: &mut Vec<u8>, padding_to: usize) {
    let padding_size = get_after_padding_size(buf.len(), padding_to) - buf.len();
    buf.extend(vec![0; padding_size]);
}
