use std::ops::{Add, AddAssign, Sub, SubAssign};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct SequenceNumber(u64);

impl<T: Into<u64>> Add<T> for SequenceNumber {
    type Output = Self;

    fn add(self, rhs: T) -> Self::Output {
        SequenceNumber(self.0 + rhs.into())
    }
}

impl<T: Into<u64>> AddAssign<T> for SequenceNumber {
    fn add_assign(&mut self, rhs: T) {
        self.0 += rhs.into();
    }
}

impl Add<SequenceNumber> for SequenceNumber {
    type Output = Self;

    fn add(self, rhs: SequenceNumber) -> Self::Output {
        SequenceNumber(self.0 + rhs.0)
    }
}

impl AddAssign<SequenceNumber> for SequenceNumber {
    fn add_assign(&mut self, rhs: SequenceNumber) {
        self.0 += rhs.0;
    }
}

impl<T: Into<u64>> Sub<T> for SequenceNumber {
    type Output = Self;

    fn sub(self, rhs: T) -> Self::Output {
        SequenceNumber(self.0 - rhs.into())
    }
}

impl Sub<SequenceNumber> for SequenceNumber {
    type Output = Self;

    fn sub(self, rhs: SequenceNumber) -> Self::Output {
        SequenceNumber(self.0 - rhs.0)
    }
}

impl<T: Into<u64>> SubAssign<T> for SequenceNumber {
    fn sub_assign(&mut self, rhs: T) {
        self.0 -= rhs.into();
    }
}

impl<T: Into<u64>> From<T> for SequenceNumber {
    fn from(value: T) -> Self {
        SequenceNumber(value.into())
    }
}

impl SubAssign<SequenceNumber> for SequenceNumber {
    fn sub_assign(&mut self, rhs: SequenceNumber) {
        self.0 -= rhs.0;
    }
}

impl SequenceNumber {
    pub fn new(round: u16, number: u16) -> Self {
        SequenceNumber(round as u64 * (u16::MAX as u64 + 1) + number as u64)
    }

    pub fn value(&self) -> u64 {
        self.0
    }

    pub fn next(&self) -> u16 {
        let mut new = SequenceNumber::new(self.round(), self.number());
        new.add_number(1);
        new.number()
    }

    pub fn round(&self) -> u16 {
        (self.0 / (u16::MAX as u64 + 1)) as u16
    }

    pub fn number(&self) -> u16 {
        (self.0 % (u16::MAX as u64 + 1)) as u16
    }

    pub fn add_round(&mut self, round: u16) {
        self.0 += round as u64 * (u16::MAX as u64 + 1)
    }

    pub fn add_number(&mut self, number: u16) {
        self.0 += number as u64
    }

    pub fn set_round(&mut self, round: u16) {
        self.0 = self.number() as u64;
        self.add_round(round);
    }

    pub fn set_number(&mut self, number: u16) {
        let round = self.round();
        self.0 = 0;
        self.set_round(round);
        self.add_number(number);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_sequence_number() {
        let seq = SequenceNumber::new(1, 2);
        assert_eq!(seq.round(), 1);
        assert_eq!(seq.number(), 2);
        assert_eq!(seq.value(), u16::MAX as u64 + 3);

        let seq = SequenceNumber::new(0, u16::MAX);
        assert_eq!(seq.round(), 0);
        assert_eq!(seq.number(), u16::MAX);
        assert_eq!(seq.value(), u16::MAX as u64);

        let seq = SequenceNumber::new(1, 0);
        assert_eq!(seq.round(), 1);
        assert_eq!(seq.number(), 0);
        assert_eq!(seq.value(), (u16::MAX as u64 + 1));

        let seq = SequenceNumber::new(1, 1);
        assert_eq!(seq.round(), 1);
        assert_eq!(seq.number(), 1);
        assert_eq!(seq.value(), u16::MAX as u64 + 2);

        let seq = SequenceNumber::new(1, u16::MAX);
        assert_eq!(seq.round(), 1);
        assert_eq!(seq.number(), u16::MAX);
        assert_eq!(seq.value(), 2 * u16::MAX as u64 + 1);

        let seq = SequenceNumber::new(2, 0);
        assert_eq!(seq.round(), 2);
        assert_eq!(seq.number(), 0);
        assert_eq!(seq.value(), 2 * u16::MAX as u64 + 2);

        let seq = SequenceNumber::new(2, 1);
        assert_eq!(seq.round(), 2);
        assert_eq!(seq.number(), 1);
        assert_eq!(seq.value(), 2 * u16::MAX as u64 + 3);

        let seq = SequenceNumber::new(2, u16::MAX);
        assert_eq!(seq.round(), 2);
        assert_eq!(seq.number(), u16::MAX);
        assert_eq!(seq.value(), 3 * u16::MAX as u64 + 2);

        let seq = SequenceNumber::new(3, 0);
        assert_eq!(seq.round(), 3);
        assert_eq!(seq.number(), 0);
        assert_eq!(seq.value(), 3 * u16::MAX as u64 + 3);

        let seq = SequenceNumber::new(u16::MAX, u16::MAX);
        assert_eq!(seq.round(), u16::MAX);
        assert_eq!(seq.number(), u16::MAX);
        assert_eq!(
            seq.value(),
            u16::MAX as u64 * u16::MAX as u64 + u16::MAX as u64 + u16::MAX as u64
        );
    }

    #[test]
    fn test_sequence_number_add() {
        let seq = SequenceNumber::new(1, 2);
        let seq = seq + 1_u16;
        assert_eq!(seq.round(), 1);
        assert_eq!(seq.number(), 3);
        assert_eq!(seq.value(), u16::MAX as u64 + 4);

        let seq = SequenceNumber::new(1, 2);
        let seq = seq + u16::MAX as u64;
        assert_eq!(seq.round(), 2);
        assert_eq!(seq.number(), 1);
        assert_eq!(seq.value(), 2 * u16::MAX as u64 + 3);

        let seq = SequenceNumber::new(1, 2);
        let seq = seq + SequenceNumber::new(1, 2);
        assert_eq!(seq.round(), 2);
        assert_eq!(seq.number(), 4);
        assert_eq!(seq.value(), 2 * u16::MAX as u64 + 6);
    }

    #[test]
    fn test_sequence_number_add_assign() {
        let mut seq = SequenceNumber::new(1, 2);
        seq += 1_u16;
        assert_eq!(seq.round(), 1);
        assert_eq!(seq.number(), 3);
        assert_eq!(seq.value(), u16::MAX as u64 + 4);

        let mut seq = SequenceNumber::new(1, 2);
        seq += u16::MAX as u64;
        assert_eq!(seq.round(), 2);
        assert_eq!(seq.number(), 1);
        assert_eq!(seq.value(), 2 * u16::MAX as u64 + 3);

        let mut seq = SequenceNumber::new(1, 2);
        seq += SequenceNumber::new(1, 2);
        assert_eq!(seq.round(), 2);
        assert_eq!(seq.number(), 4);
        assert_eq!(seq.value(), 2 * u16::MAX as u64 + 6);
    }

    #[test]
    fn test_sequence_number_sub() {
        let seq = SequenceNumber::new(1, 2);
        let seq = seq - 1_u32;
        assert_eq!(seq.round(), 1);
        assert_eq!(seq.number(), 1);
        assert_eq!(seq.value(), u16::MAX as u64 + 2);

        let seq = SequenceNumber::new(1, 2);
        let seq = seq - u16::MAX as u64;
        assert_eq!(seq.round(), 0);
        assert_eq!(seq.number(), 3);
        assert_eq!(seq.value(), 3);

        let seq = SequenceNumber::new(1, 2);
        let seq = seq - SequenceNumber::new(1, 1);
        assert_eq!(seq.round(), 0);
        assert_eq!(seq.number(), 1);
        assert_eq!(seq.value(), 1);
    }

    #[test]
    fn test_sequence_number_sub_assign() {
        let mut seq = SequenceNumber::new(1, 2);
        seq -= 1_u32;
        assert_eq!(seq.round(), 1);
        assert_eq!(seq.number(), 1);
        assert_eq!(seq.value(), u16::MAX as u64 + 2);

        let mut seq = SequenceNumber::new(1, 2);
        seq -= u16::MAX as u64;
        assert_eq!(seq.round(), 0);
        assert_eq!(seq.number(), 3);
        assert_eq!(seq.value(), 3);

        let mut seq = SequenceNumber::new(1, 2);
        seq -= SequenceNumber::new(1, 1);
        assert_eq!(seq.round(), 0);
        assert_eq!(seq.number(), 1);
        assert_eq!(seq.value(), 1);
    }

    #[test]
    fn test_sequence_number_add_round() {
        let mut seq = SequenceNumber::new(1, 2);
        seq.add_round(1);
        assert_eq!(seq.round(), 2);
        assert_eq!(seq.number(), 2);
        assert_eq!(seq.value(), 2 * (u16::MAX as u64 + 1) + 2);

        let mut seq = SequenceNumber::new(0, u16::MAX);
        seq.add_round(1);
        assert_eq!(seq.round(), 1);
        assert_eq!(seq.number(), u16::MAX);
        assert_eq!(seq.value(), (u16::MAX as u64 + 1) * 2 - 1);

        let mut seq = SequenceNumber::new(2, 0);
        seq.add_round(2);
        assert_eq!(seq.round(), 4);
        assert_eq!(seq.number(), 0);
        assert_eq!(seq.value(), 4 * (u16::MAX as u64 + 1));
    }

    #[test]
    fn test_sequence_number_add_number() {
        let mut seq = SequenceNumber::new(1, 2);
        seq.add_number(1);
        assert_eq!(seq.round(), 1);
        assert_eq!(seq.number(), 3);
        assert_eq!(seq.value(), u16::MAX as u64 + 4);

        let mut seq = SequenceNumber::new(1, u16::MAX - 1);
        seq.add_number(1);
        assert_eq!(seq.round(), 1);
        assert_eq!(seq.number(), u16::MAX);
        assert_eq!(seq.value(), 2 * u16::MAX as u64 + 1);

        let mut seq = SequenceNumber::new(1, u16::MAX);
        seq.add_number(1);
        assert_eq!(seq.round(), 2);
        assert_eq!(seq.number(), 0);
        assert_eq!(seq.value(), 2 * (u16::MAX as u64 + 1));
    }

    #[test]
    fn test_sequence_number_set_round() {
        let mut seq = SequenceNumber::new(1, 2);
        seq.set_round(3);
        assert_eq!(seq.round(), 3);
        assert_eq!(seq.number(), 2);
        assert_eq!(seq.value(), 3 * (u16::MAX as u64 + 1) + 2);

        let mut seq = SequenceNumber::new(0, u16::MAX);
        seq.set_round(1);
        assert_eq!(seq.round(), 1);
        assert_eq!(seq.number(), u16::MAX);
        assert_eq!(seq.value(), (u16::MAX as u64 + 1) * 2 - 1);

        let mut seq = SequenceNumber::new(2, 0);
        seq.set_round(4);
        assert_eq!(seq.round(), 4);
        assert_eq!(seq.number(), 0);
        assert_eq!(seq.value(), 4 * (u16::MAX as u64 + 1));
    }

    #[test]
    fn test_sequence_number_set_number() {
        let mut seq = SequenceNumber::new(1, 2);
        seq.set_number(3);
        assert_eq!(seq.round(), 1);
        assert_eq!(seq.number(), 3);
        assert_eq!(seq.value(), u16::MAX as u64 + 4);

        let mut seq = SequenceNumber::new(1, u16::MAX - 1);
        seq.set_number(u16::MAX);
        assert_eq!(seq.round(), 1);
        assert_eq!(seq.number(), u16::MAX);
        assert_eq!(seq.value(), 2 * u16::MAX as u64 + 1);

        let mut seq = SequenceNumber::new(1, u16::MAX);
        seq.set_number(0);
        assert_eq!(seq.round(), 1);
        assert_eq!(seq.number(), 0);
        assert_eq!(seq.value(), u16::MAX as u64 + 1);
    }
}
