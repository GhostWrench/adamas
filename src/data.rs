//! Data / Datum definitions

use std::result::Result;
use std::collections::HashMap;

use crate::accum::{Digit, SignedDigit, SignedDoubleDigit};
use crate::accum::Accumulator;

pub enum Length {
    Fixed(usize),    // Parameter indicates total size
    Variable(usize), // Parameter indicates maximum size
}

/// Sequence type which defines a sequence of Datum which it knows how to 
/// compress into an accumulator
pub struct Sequence<'a, T> {
    spec: &'a dyn DatumSpec<T>,
    length: Length,
}

impl<'a, T> Sequence<'a, T> {

    pub fn new(spec: &'a dyn DatumSpec<T>, length: Length) -> Self {
        Sequence { spec, length }
    }

    pub fn compress(&self, values: &[T], accum: &mut Accumulator) {
        match self.length {
            Length::Fixed(length) => self.compress_fixed(values, accum, length),
            Length::Variable(length) => self.compress_variable(values, accum, length),
        }
    }

    fn compress_fixed(&self, values: &[T], accum: &mut Accumulator, length: usize) {
        let required_bits = self.spec.permutations();
        for ii in 0..length {
            accum.mul(required_bits);
            accum.add(self.spec.encode(&values[ii]).unwrap());
        }
    }

    fn compress_variable(&self, values: &[T], accum: &mut Accumulator, max_length: usize) {
        let required_bits = self.spec.permutations() + 1;
        let count = values.len();
        if count > max_length {
            panic!("Value of length {} was not able to fit in Sequence with max length {}", count, max_length);
        }
        for ii in 0..count {
            accum.mul(required_bits);
            accum.add(self.spec.encode(&values[ii]).unwrap() + 1);
        }
        accum.mul(required_bits); // Zero to indicate end of sequence
    }

    pub fn decompress(&self, accum: &mut Accumulator) -> Vec<T> {
        match self.length {
            Length::Fixed(length) => self.decompress_fixed(accum, length),
            Length::Variable(length) => self.decompress_variable(accum, length),
        }
    }

    fn decompress_fixed(&self, accum: &mut Accumulator, length: usize) -> Vec<T> {
        let required_bits = self.spec.permutations();
        let mut decompressed: Vec<T> = Vec::with_capacity(length);
        for _ in 0..length {
            decompressed.push(self.spec.decode(accum.div(required_bits)).unwrap());
        }
        decompressed.reverse();
        decompressed
    }

    fn decompress_variable(&self, accum: &mut Accumulator, length: usize) -> Vec<T> {
        let required_bits = self.spec.permutations() + 1;
        let mut decompressed: Vec<T> = Vec::with_capacity(length);
        for _ in 0..length {
            let coded_value = accum.div(required_bits);
            if coded_value == 0 {
                break;
            }
            decompressed.push(self.spec.decode(coded_value-1).unwrap());
        }
        decompressed.reverse();
        decompressed
    }
}


/// Trait used to define a piece of data that can be compressed to a small 
/// binary representation
pub trait DatumSpec<T> {
    fn permutations(&self) -> Digit;
    fn encode(&self, input: &T) -> Result<Digit, &str>;
    fn decode(&self, value: Digit) -> Result<T, &str>;
}

/// Boolean type specification
pub struct Bool {}

impl Bool {

    pub fn new() -> Self {
        Self {}
    }
}

impl DatumSpec<bool> for Bool {

    fn permutations(&self) -> Digit {
        2
    }

    fn encode(&self, input: &bool) -> Result<Digit, &str> {
        Ok(input.clone() as Digit)
    }

    fn decode(&self, input: Digit) -> Result<bool, &str> {
        match input {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err("Could not decode the given value as boolean")
        }
    }
}

/// Integer Range type specification
pub struct IntRange {
    min: SignedDigit,
    max: SignedDigit,
}

impl IntRange {
    pub fn new(min: SignedDigit, max: SignedDigit) -> Self {
        if min < (SignedDigit::MIN + 1) {
            panic!("IntRange min cannot be less than SignedDigit::MIN + 1");
        }
        // Check if input is valid
        if min >= max {
            panic!("IntRange min may not be greater than or equal to the max");
        }
        Self {min, max}
    }

    pub fn new_full() -> Self {
        Self::new(SignedDigit::MIN+1, SignedDigit::MAX)
    }
}

impl DatumSpec<SignedDigit> for IntRange {

    fn permutations(&self) -> Digit {
        (self.max - self.min + 1) as Digit
    }

    fn encode(&self, input: &SignedDigit) -> Result<Digit, &str> {
        if *input < self.min || *input > self.max {
            return Err("Value to encode is outside allowed range");
        }
        Ok((*input - self.min) as Digit)
    }

    fn decode(&self, input: Digit) -> Result<SignedDigit, &str> {
        if input >= self.permutations() {
            return Err("Cannot decode data, input larger than possible permutations");
        }
        let result = (input as SignedDoubleDigit + self.min as SignedDoubleDigit) as SignedDigit;
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
            if lookup.contains_key(&charset[ii]) {
                panic!("Attempted to add duplicate characters to CharSet data");
            }
            lookup.insert(charset[ii], ii);
        }
        Self { charset, lookup }
    }

    pub fn lowercase_letter() -> Self {
        Self::new("abcdefghijklmnopqrstuvwxyz")
    }

    pub fn lowercase_ascii() -> Self {
        Self::new("abcdefghijklmnopqrstuvwxyz .!?0123456789()&@#$%:;'\"")
    }

    pub fn uppercase_letter() -> Self {
        Self::new("ABCDEFGHIJKLMNOPQRSTUVWXYZ")
    }

    pub fn uppercase_ascii() -> Self {
        Self::new("ABCDEFGHIJKLMNOPQRSTUVWXYZ .!?0123456789()&@#$%:;'\"")
    }

}

impl DatumSpec<char> for CharSet {

    fn permutations(&self) -> Digit {
        self.charset.len() as Digit
    }

    fn encode(&self, input: &char) -> Result<Digit, &str> {
        let value = self.lookup.get(&input);
        match value {
            None => Err("Could not encode character not defined in the character set"),
            Some(value) => Ok(value.clone() as Digit)
        }
    }

    fn decode(&self, input: Digit) -> Result<char, &str> {
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
            let new_option = String::from(options[ii]);
            if lookup.contains_key(&new_option) {
                panic!("Attempted to add duplicate strings to Enum data");
            }
            lookup.insert(new_option, ii);
        }
        Self{ options, lookup }
    }
}

impl DatumSpec<String> for Enum {

    fn permutations(&self) -> Digit {
        self.options.len() as Digit
    }

    fn encode(&self, input: &String) -> Result<Digit, &str> {
        let value = self.lookup.get(input);
        match value {
            None => Err("Given value not contained in this Enum type"),
            Some(value) => Ok(value.clone() as Digit),
        }
    }

    fn decode(&self, input: Digit) -> Result<String, &str> {
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
        assert_eq!(b.encode(&false).unwrap(), 0);
        assert_eq!(b.encode(&true).unwrap(), 1);
        assert_eq!(b.decode(0).unwrap(), false);
        assert_eq!(b.decode(1).unwrap(), true);
        assert!(b.decode(2).is_err());
    }

    #[test]
    fn range() {
        let r = IntRange::new(-10, 10);
        assert_eq!(r.permutations(), 21);
        // low values
        assert_eq!(r.encode(&-10).unwrap(), 0);
        assert_eq!(r.decode(0).unwrap(), -10);
        // mid values
        assert_eq!(r.encode(&0).unwrap(), 10);
        assert_eq!(r.decode(10).unwrap(), 0);
        // high values
        assert_eq!(r.encode(&10).unwrap(), 20);
        assert_eq!(r.decode(20).unwrap(), 10);
    }

    #[test]
    fn charset() {
        let cs = CharSet::new("abcあいうえお123$正體字");
        assert_eq!(cs.permutations(), 15);
        assert_eq!(cs.encode(&'あ').unwrap(), 3);
        assert_eq!(cs.encode(&'1').unwrap(), 8);
        assert_eq!(cs.encode(&'字').unwrap(), 14);
        assert_eq!(cs.decode(0).unwrap(), 'a');
        assert_eq!(cs.decode(7).unwrap(), 'お');
        assert_eq!(cs.decode(14).unwrap(), '字');
        assert!(cs.encode(&'0').is_err());
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
        assert_eq!(e.encode(&String::from("Banana")).unwrap(), 0);
        assert_eq!(e.encode(&String::from("Orange")).unwrap(), 1);
        assert_eq!(e.encode(&"Apple".to_string()).unwrap(), 2);
        assert!(e.encode(&"Mango".to_string()).is_err());
        assert_eq!(e.decode(0).unwrap(), "Banana");
        assert_eq!(e.decode(1).unwrap(), "Orange");
        assert_eq!(e.decode(2).unwrap(), "Apple");
        assert!(e.decode(3).is_err());
    }
}
