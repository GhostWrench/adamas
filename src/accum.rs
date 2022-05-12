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
use std::mem::transmute;

#[derive(Debug)]
pub struct Accumulator {
    data: Vec<u32>,
}

impl Accumulator {

    /// Create a new Accumulator struct
    pub fn new() -> Self {
        Self {data: Vec::new()}
    }

    /// Get the length of the internal u32 array used to store the accumulated
    /// data
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Add a value with a certain digit offset in the accumulator.
    /// 
    /// Example using the base 10 equivalent:
    /// +200 (add 2 to third digit) vs. +2 (add 2 to first digit) 
    fn add_at_place(&mut self, value: u32, place: usize) {
        let mut carry: u64 = value as u64;
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
                self.data.push(carry as u32);
                carry = 0;
            } else {
                let result: u64 = carry + self.data[vec_index] as u64;
                let [lsb, msb]: [u32; 2] = unsafe { transmute(result) };
                self.data[vec_index] = lsb;
                carry = msb as u64;
                vec_index += 1;
            }
        }
    }

    /// Add a value to the accumulator
    pub fn add(&mut self, value: u32) {
        self.add_at_place(value, 0);
    }

    /// Multiply the accumulator by a value
    pub fn mul(&mut self, value: u32) {
        // Multiply digit by digit starting with the most significant
        for ii in (0..self.len()).rev() {
            let result: u64 = (self.data[ii] as u64) * (value as u64);
            let [lsb, msb]: [u32; 2] = unsafe { transmute(result) };
            self.add_at_place(msb, ii+1);
            self.data[ii] = lsb;
        }
    }

    /// Divide the accumulator by a value and return the remainder
    pub fn div(&mut self, value: u32) -> u32 {
        // Check for divide by zero and panic
        if value == 0 {
            panic!("Cannot divide by zero!");
        }
        let den: u64 = value as u64;
        let mut rem: [u32; 2] = [0; 2];
        for ii in (0..self.len()).rev() {
            let num: [u32; 2] = [self.data[ii], rem[0]];
            let result: [u32; 2];
            unsafe {
                let num: u64 = transmute(num);
                result = transmute(num / den);
                rem = transmute(num % den);
            }
            if result[0] == 0 {
                self.data.pop();
            }
            else {
                self.data[ii] = result[0];
            }
        }
        rem[0]
    }

    /// Retrieve the contents of the accumulator as a hex string
    pub fn to_hex_str(&self) -> String {
        let mut s: String = String::from("");
        for digit in self.data.iter().rev() {
            s.push_str(format!("{:08x} ", digit).as_str());
        }
        s.pop();
        s
    }
}

#[cfg(test)]
mod tests {

    use crate::accum::Accumulator;

    #[test]
    fn add_at_place() {
        // test some basic addition
        let mut a = Accumulator::new();
        a.add_at_place(3, 2);
        assert_eq!(a.to_hex_str(), "00000003 00000000 00000000");
        a.add_at_place(2, 1);
        assert_eq!(a.to_hex_str(), "00000003 00000002 00000000");
        a.add_at_place(1, 0);
        assert_eq!(a.to_hex_str(), "00000003 00000002 00000001");
        a.add_at_place(u32::MAX, 1);
        assert_eq!(a.to_hex_str(), "00000004 00000001 00000001");
        // test that carry propigates
        let mut a = Accumulator::new();
        a.add_at_place(u32::MAX, 0);
        a.add_at_place(u32::MAX, 1);
        a.add_at_place(u32::MAX, 2);
        assert_eq!(a.to_hex_str(), "ffffffff ffffffff ffffffff");
        a.add(1);
        assert_eq!(a.to_hex_str(), "00000001 00000000 00000000 00000000");
        a.add_at_place(u32::MAX, 3);
        assert_eq!(a.to_hex_str(), "00000001 00000000 00000000 00000000 00000000");
    }

    #[test]
    fn add() {
        // test very basic addition and vector grows
        let mut a = Accumulator::new();
        a.add(2);
        a.add(4);
        assert_eq!(a.to_hex_str(), "00000006");
        a.add(u32::MAX);
        assert_eq!(a.to_hex_str(), "00000001 00000005");
        a.add(0xf0);
        assert_eq!(a.to_hex_str(), "00000001 000000f5");
    }

    #[test]
    fn mul() {
        let mut a = Accumulator::new();
        a.add_at_place(0xF0000000, 0);
        a.add_at_place(0xF0000000, 1);
        a.mul(2);
        assert_eq!(a.to_hex_str(), "00000001 e0000001 e0000000");

        let mut a = Accumulator::new();
        a.add_at_place(u32::MAX, 2);
        a.add_at_place(u32::MAX, 1);
        a.add_at_place(u32::MAX, 0);
        a.mul(u32::MAX);
        assert_eq!(a.to_hex_str(), "fffffffe ffffffff ffffffff 00000001");
    }

    #[test]
    fn div() {
        let mut a = Accumulator::new();
        a.add(0xff);
        let r = a.div(2);
        assert_eq!(r, 1);
        assert_eq!(a.to_hex_str(), "0000007f");

        let mut a = Accumulator::new();
        let encode: [u32; 4] = [u32::MAX-10000, u32::MAX-1000, u32::MAX-100, u32::MAX-10];
        let mut decode: [u32; 4] = [0; 4];
        for ii in 0..encode.len() {
            a.mul(encode[ii]+1);
            a.add(encode[ii]);
        }
        for ii in (0..4).rev() {
            decode[ii] = a.div(encode[ii]+1);
        }
        assert_eq!(encode, decode);
    }

    #[test]
    fn all_three() {
        let mut a = Accumulator::new();
        a.add(u32::MAX);
        a.mul(u32::MAX);
        a.div(u32::MAX);
        assert_eq!(a.to_hex_str(), "ffffffff");
    }
}
