//! Data / Datum definitions

use std::result::Result;
use std::collections::HashMap;

/// Trait used to define a piece of data that can be compressed to a small 
/// binary representation
pub trait DatumSpec<T> {
    fn permutations(&self) -> u32;
    fn encode(&self, input: T) -> Result<u32, &str>;
    fn decode(&self, value: u32) -> Result<T, &str>;
}

/// Boolean type specification
pub struct Bool {}

impl Bool {

    pub fn new() -> Self {
        Self {}
    }
}

impl DatumSpec<bool> for Bool {

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

/// Integer Range type specification
pub struct IntRange {
    min: i32,
    max: i32,
}

impl IntRange {
    pub fn new(min: i32, max: i32) -> Self {
        if min < (i32::MIN + 1) {
            panic!("IntRange min cannot be less than i32::MIN + 1");
        }
        // Check if input is valid
        if min >= max {
            panic!("IntRange min may not be greater than or equal to the max");
        }
        Self {min, max}
    }

    pub fn new_full() -> Self {
        Self::new(i32::MIN+1, i32::MAX)
    }
}

impl DatumSpec<i32> for IntRange {

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


/// CharSet type specification
pub struct CharSet {
    charset: Vec<char>,
    lookup: HashMap<char, usize>,
}

impl CharSet {

    pub fn new(charset: &'static str) -> Self {
        let charset: Vec<char> = charset.chars().collect();
        let mut lookup: HashMap<char, usize> = HashMap::with_capacity(charset.len());
        for ii in 0..charset.len() {
            lookup.insert(charset[ii], ii);
        }
        Self { charset, lookup }
    }

    pub fn lowercase_ascii() -> Self {
        Self::new("abcdefghijklmnopqrstuvwxyz .!?0123456789()&@#$%:;'\"")
    }

    pub fn upercase_ascii() -> Self {
        Self::new("ABCDEFGHIJKLMNOPQRSTUVWXYZ .!?0123456789()&@#$%:;'\"")
    }

}

impl DatumSpec<char> for CharSet {

    fn permutations(&self) -> u32 {
        self.charset.len() as u32
    }

    fn encode(&self, input: char) -> Result<u32, &str> {
        let value = self.lookup.get(&input);
        match value {
            None => Err("Could not encode character not defined in the character set"),
            Some(value) => Ok(*value as u32)
        }
    }

    fn decode(&self, input:u32) -> Result<char, &str> {
        let index = input as usize;
        if index >= self.charset.len() {
            Err("Could not decode value to a character")
        } else {
            let result = self.charset[index];
            Ok(result)
        }
    }
}

/// Enumeration type specification
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

impl DatumSpec<String> for Enum {

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

    use crate::data::{DatumSpec, Bool, IntRange, CharSet, Enum};

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
    fn range() {
        let r = IntRange::new(-10, 10);
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
    fn charset() {
        let cs = CharSet::new("abcあいうえお123$正體字");
        assert_eq!(cs.permutations(), 15);
        assert_eq!(cs.encode('あ').unwrap(), 3);
        assert_eq!(cs.encode('1').unwrap(), 8);
        assert_eq!(cs.encode('字').unwrap(), 14);
        assert_eq!(cs.decode(0).unwrap(), 'a');
        assert_eq!(cs.decode(7).unwrap(), 'お');
        assert_eq!(cs.decode(14).unwrap(), '字');
        assert!(cs.encode('0').is_err());
        assert!(cs.decode(15).is_err());
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