//! Data / Datum definitions

use std::result::Result;
use std::collections::HashMap;

use crate::accum::{Digit, SignedDigit, SignedDoubleDigit};
use crate::accum::Accumulator;

/// SequenceLength: indicate a fixed length or a variable length with a maximum
pub enum SequenceLength {
    Fixed(usize),    // Parameter indicates total size
    Variable(usize), // Parameter indicates maximum size
}

/// Sequence type which defines a sequence of Datum which it knows how to 
/// compress into an accumulator
pub struct Sequencer<'a, T> {
    spec: &'a dyn DatumSpec<T>,
    length: SequenceLength,
}

impl<'a, T> Sequencer<'a, T> {

    pub fn new(spec: &'a dyn DatumSpec<T>, length: SequenceLength) -> Self {
        Self { spec, length }
    }

    pub fn compress(&self, values: &[T], accum: &mut Accumulator) {
        match self.length {
            SequenceLength::Fixed(length) => self.compress_fixed(values, accum, length),
            SequenceLength::Variable(length) => self.compress_variable(values, accum, length),
        }
    }

    fn compress_fixed(&self, values: &[T], accum: &mut Accumulator, length: usize) {
        let permutations = self.spec.permutations();
        for ii in 0..length {
            accum.mul(permutations);
            accum.add(self.spec.encode(&values[ii]).unwrap());
        }
    }

    fn compress_variable(&self, values: &[T], accum: &mut Accumulator, max_length: usize) {
        let permutations = self.spec.permutations() + 1;
        let count = values.len();
        if count > max_length {
            panic!("Value of length {} was not able to be compressed by Sequencer with max length {}", count, max_length);
        }
        accum.mul(permutations); // Zero to indicate end of sequence
        for ii in 0..count {
            accum.mul(permutations);
            accum.add(self.spec.encode(&values[ii]).unwrap() + 1);
        }
    }

    pub fn decompress(&self, accum: &mut Accumulator) -> Vec<T> {
        match self.length {
            SequenceLength::Fixed(length) => self.decompress_fixed(accum, length),
            SequenceLength::Variable(length) => self.decompress_variable(accum, length),
        }
    }

    fn decompress_fixed(&self, accum: &mut Accumulator, length: usize) -> Vec<T> {
        let permutations = self.spec.permutations();
        let mut decompressed: Vec<T> = Vec::with_capacity(length);
        for _ in 0..length {
            decompressed.push(self.spec.decode(accum.div(permutations)).unwrap());
        }
        decompressed.reverse();
        decompressed
    }

    fn decompress_variable(&self, accum: &mut Accumulator, length: usize) -> Vec<T> {
        let permutations = self.spec.permutations() + 1;
        let mut decompressed: Vec<T> = Vec::with_capacity(length);
        for _ in 0..length {
            let coded_value = accum.div(permutations);
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

/// Number range in fixed point format
/// 
/// Note: The compression used by this data type is not lossless, also the 
///       provided minimum and maximum values are not guaranteed to be exact
pub struct FixedPointRange {
    min: SignedDigit,
    max: SignedDigit,
    decimals: u32,
}

impl FixedPointRange {

    pub fn new(min: f64, max: f64, decimals: u32) -> Self {
        // Calculate the absolute maximum values from the required decimals
        let abs_max_fixed = SignedDigit::MAX >> decimals;
        let abs_max_float = abs_max_fixed as f64;
        if max > abs_max_float {
            panic!("Max allowable value for {} binary decimals is {}", decimals, abs_max_float);
        }
        if min < -abs_max_float {
            panic!("Min allowable value for {} binary decimals is {}", decimals, -abs_max_float);
        }
        let min = float2fixed(min, decimals);
        let max = float2fixed(max, decimals);
        Self { min, max, decimals }
    }
}

impl DatumSpec<f64> for FixedPointRange {

    fn permutations(&self) -> Digit {
        ((self.max - self.min) + 1) as Digit
    }

    fn encode(&self, input: &f64) -> Result<Digit, &str> {
        let num = float2fixed(*input, self.decimals);
        if num < self.min {
            Err("Number is to small to be encoded as a fixed point")
        } else if num > self.max {
            Err("Number is too big to be encoded as a fixed point")
        } else {
            let encoded_num = (num - self.min) as Digit;
            Ok(encoded_num)
        }
    }

    fn decode(&self, input: Digit) -> Result<f64, &str> {
        if input >= self.permutations() {
            return Err("Cannot decode data, input larger than possible permutations");
        }
        let num = (input as SignedDigit) + self.min;
        Ok(fixed2float(num, self.decimals))
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

// Utility functions

/// Convert a floating point number to a fixed point number
fn float2fixed(value: f64, decimals: u32) -> SignedDigit {
    let abs_max_fixed = SignedDigit::MAX >> decimals;
    let two: f64 = 2.0;
    let mut value = (value * two.powi(decimals as i32)) as SignedDigit;
    if value < -abs_max_fixed {
        value = -abs_max_fixed;
    } else if value > abs_max_fixed {
        value = abs_max_fixed;
    }
    value
}

fn fixed2float(value: SignedDigit, decimals: u32) -> f64 {
    let two: f64 = 2.0;
    (value as f64) / two.powi(decimals as i32)
}

#[cfg(test)]
mod tests {

    //use std::vec::Vec;

    use crate::data::{
        DatumSpec, 
        Bool, 
        IntRange, 
        FixedPointRange, 
        CharSet, 
        Enum,
        SequenceLength,
        Sequencer,
    };
    
    use crate::accum::Accumulator;

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
    fn int_range() {
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
    fn fixed_point_range() {
        let r = FixedPointRange::new(-255.99, 255.99, 2);
        assert_eq!(r.permutations(), 2047);
        // low values
        assert_eq!(r.encode(&-255.76).unwrap(), 0);
        assert_eq!(r.decode(0).unwrap(), -255.75);
        // mid values
        assert_eq!(r.encode(&0.0).unwrap(), 1023);
        assert_eq!(r.decode(1023).unwrap(), 0.0);
        assert_eq!(r.encode(&-0.25).unwrap(), 1022);
        assert_eq!(r.encode(&0.25).unwrap(), 1024);
        // high values
        assert_eq!(r.encode(&255.75).unwrap(), 2046);
        assert_eq!(r.decode(2046).unwrap(), 255.75);
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

    #[test]
    fn seq_bool() {
        // Fixed length sequence
        let mut a = Accumulator::new();
        let spec = Bool::new();
        let seq = &[false, true, true, false, true];
        // Sequence as fixed
        let sequencer0 = Sequencer::new(&spec, SequenceLength::Fixed(5));
        sequencer0.compress(seq, &mut a);
        // Sequence as variable length
        let sequencer1 = Sequencer::new(&spec, SequenceLength::Variable(20));
        sequencer1.compress(seq, &mut a);
        // Decompress and compare
        let deseq = sequencer1.decompress(&mut a);
        assert_eq!(deseq.as_slice(), seq);
        let deseq = sequencer0.decompress(&mut a);
        assert_eq!(deseq.as_slice(), seq);
    }

    #[test]
    fn seq_int_range() {
        let mut a = Accumulator::new();
        let seq = &[5, 10, -1, 3];
        // Sequence as variable
        let spec0 = IntRange::new(-1, 10);
        let sequencer0 = Sequencer::new(
            &spec0,
            SequenceLength::Variable(50),
        );
        sequencer0.compress(seq, &mut a);
        // Sequence as fixed
        let spec1 = IntRange::new(-2, 20);
        let sequencer1 = Sequencer::new(
            &spec1,
            SequenceLength::Fixed(4),
        );
        sequencer1.compress(seq, &mut a);
        // Decompress
        let deseq = sequencer1.decompress(&mut a);
        assert_eq!(deseq.as_slice(), seq);
        let deseq = sequencer0.decompress(&mut a);
        assert_eq!(deseq.as_slice(), seq);
    }

    #[test]
    fn seq_fixed_point_range() {
        let mut a = Accumulator::new();
        let spec = FixedPointRange::new(-256.0, 256.0, 3);
        // Sequence 0
        let seq0: &[f64] = &[-100.0, -255.875, 255.875, 0.0, 0.125, 123.625];
        let sequencer0 = Sequencer::new(
            &spec,
            SequenceLength::Fixed(6),
        );
        sequencer0.compress(seq0, &mut a);
        // Sequence 1
        let seq1: &[f64] = &[0.0, 0.125, -255.875, 255.875];
        let sequencer1 = Sequencer::new(
            &spec,
            SequenceLength::Variable(100),
        );
        sequencer1.compress(seq1, &mut a);
        // Decompress
        let deseq = sequencer1.decompress(&mut a);
        assert_eq!(deseq.as_slice(), seq1);
        let deseq = sequencer0.decompress(&mut a);
        assert_eq!(deseq.as_slice(), seq0);
    }
}
