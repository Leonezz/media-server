use std::{io, string};

use thiserror::Error;

use crate::amf3::{self};

#[derive(Error, Debug)]
pub enum AmfReadError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("invalid utf8 data: {0}")]
    InvalidUtf8(#[from] string::FromUtf8Error),
    #[error("unsupported amf value marker: {marker}")]
    Unsupported { marker: u8 },
    #[error("unknown marker: {marker}")]
    Unknown { marker: u8 },
    #[error("index of reference out of range, index: {index}")]
    OutOfRangeReference { index: usize },
    #[error("circular reference not supported, index: {index}")]
    CircularReference { index: usize },
    #[error("unexpected timezone in amf0: offset: {offset}")]
    UnexpectedTimeZone { offset: i16 },
    #[error("invalid value for a unix date: {milliseconds}")]
    InvalidDate { milliseconds: f64 },
    #[error("Unsupported externalizable data, name: {name}")]
    UnsupportedExternalizable { name: String },
}

pub type AmfReadResult<T> = Result<T, AmfReadError>;

#[derive(Error, Debug)]
pub enum AmfWriteError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("u29 value out of range, value: {value}")]
    U29OutOfRange { value: u32 },
    #[error("size value out of range, value: {value}")]
    SizeOutOfRange { value: usize },
    #[error("trait: {entries:?}, sealed_count: {sealed_count}")]
    Amf3TraitInvalid {
        entries: Vec<(String, amf3::Value)>,
        sealed_count: usize,
    },
}
pub type AmfWriteResult = Result<(), AmfWriteError>;
