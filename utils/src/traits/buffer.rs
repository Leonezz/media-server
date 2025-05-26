pub trait GenericSequencer {
    type In;
    type Out;
    type Error;
    fn enqueue(&mut self, packet: Self::In) -> Result<(), Self::Error>;
    fn try_dump(&mut self) -> Vec<Self::Out>;
}

pub trait GenericFragmentComposer {
    type In;
    type Out;
    type Error;
    fn enqueue(&mut self, packet: Self::In) -> Result<Option<Self::Out>, Self::Error>;
}
