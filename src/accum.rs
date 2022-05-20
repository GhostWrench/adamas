//! accum: Simple accumulator
//! 
//! This module provides a very simple "infinite" precision accumulator struct 
//! (Accumulator) that supports three operations: add multiply and divide. Any 
//! number of operations can be applied to the Accumulator without having to 
//! worry about an overflow, it will keep expanding. This is useful for encoding
//! compressed data
//! 
//! # Examples

use std::vec::Vec;
use std::string::String;

// Data types used as big digits
pub type Digit = u64;
pub type DoubleDigit = u128;
pub type SignedDigit = i64;
pub type SignedDoubleDigit = i128;

/// Accumulator struct, accumulates the results of chained math operations
pub struct Accumulator {
    data: Vec<Digit>,
}

impl Accumulator {

    /// Create a new Accumulator struct
    pub fn new() -> Self {
        Self {data: Vec::new()}
    }

    /// Get the length of the internal Digit array used to store the accumulated
    /// data
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Add a value with a certain digit offset in the accumulator.
    /// 
    /// Example using the base 10 equivalent:
    /// +200 (add 2 to third digit) vs. +2 (add 2 to first digit) 
    fn add_at_place(&mut self, value: Digit, place: usize) {
        let mut carry: DoubleDigit = value as DoubleDigit;
        let mut vec_index: usize = place;
        // do nothing if the value is 0
        if value == 0 {
            return;
        }
        // add digits until the accumulator is big enough 
        while vec_index > self.len() {
            self.data.push(0);
        }
        // Keep adding and carrying until the carry value is 0
        while carry != 0 {
            if vec_index == self.len() {
                self.data.push(carry as Digit);
                carry = 0;
            } else {
                let result: DoubleDigit = carry + self.data[vec_index] as DoubleDigit;
                let [lsb, msb]: [Digit; 2] = chop_digits(result);
                self.data[vec_index] = lsb;
                carry = msb as DoubleDigit;
                vec_index += 1;
            }
        }
    }

    /// Add a value to the accumulator
    pub fn add(&mut self, value: Digit) {
        self.add_at_place(value, 0);
    }

    /// Multiply the accumulator by a value
    pub fn mul(&mut self, value: Digit) {
        // Multiply digit by digit starting with the most significant
        for ii in (0..self.len()).rev() {
            let result: DoubleDigit = (self.data[ii] as DoubleDigit) * (value as DoubleDigit);
            let [lsb, msb]: [Digit; 2] = chop_digits(result);
            self.add_at_place(msb, ii+1);
            self.data[ii] = lsb;
        }
    }

    /// Divide the accumulator by a value and return the remainder
    pub fn div(&mut self, value: Digit) -> Digit {
        // Check for divide by zero and panic
        if value == 0 {
            panic!("Cannot divide by zero!");
        }
        let den: DoubleDigit = value as DoubleDigit;
        let mut rem: [Digit; 2] = [0; 2];
        for ii in (0..self.len()).rev() {
            let num: [Digit; 2] = [self.data[ii], rem[0]];
            let result: [Digit; 2];
            let num: DoubleDigit = fuse_digits(num);
            result = chop_digits(num / den);
            rem = chop_digits(num % den);
            self.data[ii] = result[0];
        }
        // Remove most significant digit if it is 0
        if let Some(msd) = self.data.last() {
            if *msd == 0 {
                self.data.pop();
            }
        }
        rem[0]
    }

    /// Shift the accumulator to the left (multiply by a power of 2)
    pub fn shl(&mut self, shift: usize) {
        // error if trying to shift more than the number of bits in Digit
        if shift > Digit::BITS as usize {
            panic!("Can not apply shift to accumulator greater than 64");
        }
        let mut carry: Digit = 0;
        // loop through digits and apply the shift
        for ii in 0..self.data.len() {
            let result: DoubleDigit = ((self.data[ii] as DoubleDigit) << shift) + carry as DoubleDigit;
            let [lsb, msb]: [Digit; 2] = chop_digits(result);
            self.data[ii] = lsb;
            carry = msb;
        }
        if carry > 0 {
            self.data.push(carry);
        }
    }

    /// Shift the accumulator to the right (divide by a factor of 2)
    pub fn shr(&mut self, shift: usize) -> Digit {
        // error if trying to shift more than the number of bits in Digit
        if shift > Digit::BITS as usize {
            panic!("Can not apply shift to accumulator greater than 64");
        }
        // loop through digits and apply the shift
        let mut carry: Digit = 0;
        for ii in (0..self.data.len()).rev() {
            let result: [Digit; 2] = [0, self.data[ii]];
            let mut result: DoubleDigit = fuse_digits(result);
            result >>= shift;
            result += (carry as DoubleDigit) << Digit::BITS;
            let [lsb, msb]: [Digit; 2] = chop_digits(result);
            carry = lsb;
            self.data[ii] = msb;
        } 
        // Remove most significant digit if it is 0
        if let Some(msd) = self.data.last() {
            if *msd == 0 {
                self.data.pop();
            }
        }
        carry >>= Digit::BITS - shift as u32;
        carry
    }

    /// Retrieve the contents of the accumulator as a hex string
    fn to_hex_str(&self) -> String {
        let mut s: String = String::from("");
        for digit in self.data.iter().rev() {
            // formatting of this string is hardcoded to match the "Digit" type
            s.push_str(format!("{:016x} ", digit).as_str());
        }
        s.pop();
        s
    }
}

/// Combine two Digits into a DoubleDigit
fn fuse_digits(digits: [Digit; 2]) -> DoubleDigit {
    // Note:
    // digits[0] is the least significant digit LSD
    // digits[1] is the most significant digit MSD
    let mut result: DoubleDigit = 0;
    result += digits[0] as DoubleDigit;
    result += (digits[1] as DoubleDigit) << Digit::BITS;
    result
}

/// Separate a DoubleDigit into two Digits
fn chop_digits(digits: DoubleDigit) -> [Digit; 2] {
    let msd: DoubleDigit = digits >> Digit::BITS;
    let lsd: DoubleDigit = digits - (msd << Digit::BITS);
    [lsd as Digit, msd as Digit]
}

#[cfg(test)]
mod tests {

    use crate::accum::Accumulator;
    use crate::accum::Digit;

    #[test]
    fn add_at_place() {
        // test some basic addition
        let mut a = Accumulator::new();
        a.add_at_place(3, 2);
        assert_eq!(a.to_hex_str(), "0000000000000003 0000000000000000 0000000000000000");
        a.add_at_place(2, 1);
        assert_eq!(a.to_hex_str(), "0000000000000003 0000000000000002 0000000000000000");
        a.add_at_place(1, 0);
        assert_eq!(a.to_hex_str(), "0000000000000003 0000000000000002 0000000000000001");
        a.add_at_place(Digit::MAX, 1);
        assert_eq!(a.to_hex_str(), "0000000000000004 0000000000000001 0000000000000001");
        // test that carry propigates
        let mut a = Accumulator::new();
        a.add_at_place(Digit::MAX, 0);
        a.add_at_place(Digit::MAX, 1);
        a.add_at_place(Digit::MAX, 2);
        assert_eq!(a.to_hex_str(), "ffffffffffffffff ffffffffffffffff ffffffffffffffff");
        a.add(1);
        assert_eq!(a.to_hex_str(), "0000000000000001 0000000000000000 0000000000000000 0000000000000000");
        a.add_at_place(Digit::MAX, 3);
        assert_eq!(a.to_hex_str(), "0000000000000001 0000000000000000 0000000000000000 0000000000000000 0000000000000000");
    }

    #[test]
    fn add() {
        // test very basic addition and vector grows
        let mut a = Accumulator::new();
        a.add(2);
        a.add(4);
        assert_eq!(a.to_hex_str(), "0000000000000006");
        a.add(Digit::MAX);
        assert_eq!(a.to_hex_str(), "0000000000000001 0000000000000005");
        a.add(0xf0);
        assert_eq!(a.to_hex_str(), "0000000000000001 00000000000000f5");
    }

    #[test]
    fn mul() {
        let mut a = Accumulator::new();
        a.add_at_place(0xF000000000000000, 0);
        a.add_at_place(0xF000000000000000, 1);
        a.mul(2);
        assert_eq!(a.to_hex_str(), "0000000000000001 e000000000000001 e000000000000000");

        let mut a = Accumulator::new();
        a.add_at_place(Digit::MAX, 2);
        a.add_at_place(Digit::MAX, 1);
        a.add_at_place(Digit::MAX, 0);
        a.mul(Digit::MAX);
        assert_eq!(a.to_hex_str(), "fffffffffffffffe ffffffffffffffff ffffffffffffffff 0000000000000001");
    }

    #[test]
    fn div() {
        let mut a = Accumulator::new();
        a.add(0xff);
        let r = a.div(2);
        assert_eq!(r, 1);
        assert_eq!(a.to_hex_str(), "000000000000007f");
        // Use divides to encode values
        let mut a = Accumulator::new();
        let encode: [Digit; 4] = [Digit::MAX-10000, Digit::MAX-1000, Digit::MAX-100, Digit::MAX-10];
        let mut decode: [Digit; 4] = [0; 4];
        for ii in 0..encode.len() {
            a.mul(encode[ii]+1);
            a.add(encode[ii]);
        }
        // test divide by 1
        let ahex = a.to_hex_str();
        a.div(1);
        assert_eq!(ahex, a.to_hex_str());
        for ii in (0..4).rev() {
            decode[ii] = a.div(encode[ii]+1);
        }
        assert_eq!(encode, decode);
    }

    #[test]
    fn all_three() {
        let mut a = Accumulator::new();
        a.add(Digit::MAX);
        a.mul(Digit::MAX);
        a.div(Digit::MAX);
        assert_eq!(a.to_hex_str(), "ffffffffffffffff");
    }

    #[test]
    fn shift() {
        let mut a = Accumulator::new();
        a.add(0xa00000000000000b);
        a.shl(4);
        assert_eq!(a.to_hex_str(), "000000000000000a 00000000000000b0");
        a.shl(64);
        assert_eq!(a.to_hex_str(), "000000000000000a 00000000000000b0 0000000000000000");
        a.shr(60);
        assert_eq!(a.to_hex_str(), "00000000000000a0 0000000000000b00");
        let rem = a.shr(12);
        assert_eq!(a.to_hex_str(), "0a00000000000000");
        assert_eq!(rem, 0xb00);
        // Use shifts to encode values
        let mut a = Accumulator::new();
        let encode: [Digit; 4] = [10, 20, 50, 250];
        let mut decode: [Digit; 4] = [0; 4];
        for ii in 0..encode.len() {
            a.shl(8);
            a.add(encode[ii]);
        }
        for ii in (0..4).rev() {
            decode[ii] = a.shr(8);
        }
        assert_eq!(encode, decode);
    }
}
