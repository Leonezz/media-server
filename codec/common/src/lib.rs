pub mod audio;
pub mod errors;
pub mod video;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameType {
    SequenceStart,
    CodedFrames,
    KeyFrame,
    SequenceEnd,
}
