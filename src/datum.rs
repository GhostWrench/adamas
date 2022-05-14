//! Data / Datum definitions

use std::result::Result;
use std::collections::HashMap;

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
        if min < (i32::MIN + 1) {
            panic!("Range min cannot be less than i32::MIN + 1");
        }
        // Check if input is valid
        if min >= max {
            panic!("Range min may not be greater than or equal to the max");
        }
        Self {min, max}
    }

    pub fn new_full() -> Self {
        Self::new(i32::MIN+1, i32::MAX)
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

/// Boolean type, encodes a boolean
pub struct Bool {}

impl Bool {

    pub fn new() -> Self {
        Self {}
    }
}

impl Datum<bool> for Bool {

    fn permutations(&self) -> u32 {
        2
    }

    fn encode(&self, input: bool) -> Result<u32, &str> {
        Ok(input as u32)
    }

    fn decode(&self, input: u32) -> Result<bool, &str> {
        match input {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err("Could not decode the given value as boolean")
        }
    }
}

/// Enumeration type, encodes a selection of different strings
pub struct Enum {
    options: &'static [&'static str],
    lookup: HashMap<String, usize>,
}

impl Enum {

    pub fn new(options: &'static [&'static str]) -> Self {
        let mut lookup = HashMap::with_capacity(options.len());
        for ii in 0..options.len() {
            lookup.insert(String::from(options[ii]), ii);
        }
        Self{ options, lookup }
    }
}

impl Datum<String> for Enum {

    fn permutations(&self) -> u32 {
        self.options.len() as u32
    }

    fn encode(&self, input: String) -> Result<u32, &str> {
        let value = self.lookup.get(&input);
        match value {
            None => Err("Given value not contained in this Enum type"),
            Some(value) => Ok(*value as u32),
        }
    }

    fn decode(&self, input: u32) -> Result<String, &str> {
        let index = input as usize;
        if index >= self.options.len() {
            Err("Could not decode value as an Enum")
        } else {
            let result = String::from(self.options[index]);
            Ok(result)
        }
    }
}


/// Tests for this module
mod tests {

    use crate::datum::{Datum, Range, Bool, Enum};

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

    #[test]
    fn bool() {
        let b = Bool::new();
        assert_eq!(b.permutations(), 2);
        assert_eq!(b.encode(false).unwrap(), 0);
        assert_eq!(b.encode(true).unwrap(), 1);
        assert_eq!(b.decode(0).unwrap(), false);
        assert_eq!(b.decode(1).unwrap(), true);
        assert!(b.decode(2).is_err());
    }

    #[test]
    fn tenum() {
        let e = Enum::new(&[
            "Banana",
            "Orange",
            "Apple"
        ]);
        assert_eq!(e.permutations(), 3);
        assert_eq!(e.encode(String::from("Banana")).unwrap(), 0);
        assert_eq!(e.encode(String::from("Orange")).unwrap(), 1);
        assert_eq!(e.encode("Apple".to_string()).unwrap(), 2);
        assert!(e.encode("Mango".to_string()).is_err());
        assert_eq!(e.decode(0).unwrap(), "Banana");
        assert_eq!(e.decode(1).unwrap(), "Orange");
        assert_eq!(e.decode(2).unwrap(), "Apple");
        assert!(e.decode(3).is_err());
    }
}
