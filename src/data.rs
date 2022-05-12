//! Data / Datum definitions

use std::result::Result;

pub trait Datum<T> {
    fn permutations(&self) -> u32;
    fn encode(&self, input: T) -> Result<u32, &str>;
    fn decode(&self, value: u32) -> Result<T, &str>;
}

/// Range type, encodes an integer range
pub struct Range {
    min: i32,
    max: i32,
}

impl Range {
    pub fn new(min: i32, max: i32) -> Self {
        if min == i32::MIN {
            panic!("Range min cannot be less than -2147483646");
        }
        // Check if input is valid
        if min >= max {
            panic!("Range min may not be greater than or equal to the max");
        }
        Self {min, max}
    }

    pub fn new_full() -> Self {
        Self::new(i32::MIN+2, i32::MAX)
    }
}

impl Datum<i32> for Range {

    fn permutations(&self) -> u32 {
        (self.max - self.min + 1) as u32
    }

    fn encode(&self, input: i32) -> Result<u32, &str> {
        if input < self.min || input > self.max {
            return Err("Value to encode is outside allowed range");
        }
        Ok((input - self.min) as u32)
    }

    fn decode(&self, input: u32) -> Result<i32, &str> {
        if input >= self.permutations() {
            return Err("Cannot decode data, input larger than possible permutations");
        }
        let result = (input as i64 + self.min as i64) as i32;
        Ok(result) 
    }
}

mod tests {

    use crate::data::{Range, Datum};

    #[test]
    fn range() {
        let r = Range::new(-10, 10);
        assert_eq!(r.permutations(), 21);
        // low values
        assert_eq!(r.encode(-10).unwrap(), 0);
        assert_eq!(r.decode(0).unwrap(), -10);
        // mid values
        assert_eq!(r.encode(0).unwrap(), 10);
        assert_eq!(r.decode(10).unwrap(), 0);
        // high values
        assert_eq!(r.encode(10).unwrap(), 20);
        assert_eq!(r.decode(20).unwrap(), 10);
    }
}
