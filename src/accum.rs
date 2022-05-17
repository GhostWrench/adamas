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

use crate::{Block, DoubleBlock};

#[derive(Debug)]
pub struct Accumulator {
    data: Vec<Block>,
}

impl Accumulator {

    /// Create a new Accumulator struct
    pub fn new() -> Self {
        Self {data: Vec::new()}
    }

    /// Get the length of the internal Block array used to store the accumulated
    /// data
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Add a value with a certain digit offset in the accumulator.
    /// 
    /// Example using the base 10 equivalent:
    /// +200 (add 2 to third digit) vs. +2 (add 2 to first digit) 
    fn add_at_place(&mut self, value: Block, place: usize) {
        let mut carry: DoubleBlock = value as DoubleBlock;
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
                self.data.push(carry as Block);
                carry = 0;
            } else {
                let result: DoubleBlock = carry + self.data[vec_index] as DoubleBlock;
                let [lsb, msb]: [Block; 2] = unsafe { transmute(result) };
                self.data[vec_index] = lsb;
                carry = msb as DoubleBlock;
                vec_index += 1;
            }
        }
    }

    /// Add a value to the accumulator
    pub fn add(&mut self, value: Block) {
        self.add_at_place(value, 0);
    }

    /// Multiply the accumulator by a value
    pub fn mul(&mut self, value: Block) {
        // Multiply digit by digit starting with the most significant
        for ii in (0..self.len()).rev() {
            let result: DoubleBlock = (self.data[ii] as DoubleBlock) * (value as DoubleBlock);
            let [lsb, msb]: [Block; 2] = unsafe { transmute(result) };
            self.add_at_place(msb, ii+1);
            self.data[ii] = lsb;
        }
    }

    /// Divide the accumulator by a value and return the remainder
    pub fn div(&mut self, value: Block) -> Block {
        // Check for divide by zero and panic
        if value == 0 {
            panic!("Cannot divide by zero!");
        }
        let den: DoubleBlock = value as DoubleBlock;
        let mut rem: [Block; 2] = [0; 2];
        for ii in (0..self.len()).rev() {
            let num: [Block; 2] = [self.data[ii], rem[0]];
            let result: [Block; 2];
            unsafe {
                let num: DoubleBlock = transmute(num);
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
    fn to_hex_str(&self) -> String {
        let mut s: String = String::from("");
        for digit in self.data.iter().rev() {
            // formatting of this string is hardcoded to match the "Block" type
            s.push_str(format!("{:016x} ", digit).as_str());
        }
        s.pop();
        s
    }
}

#[cfg(test)]
mod tests {

    use crate::accum::Accumulator;
    use crate::Block;

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
        a.add_at_place(Block::MAX, 1);
        assert_eq!(a.to_hex_str(), "0000000000000004 0000000000000001 0000000000000001");
        // test that carry propigates
        let mut a = Accumulator::new();
        a.add_at_place(Block::MAX, 0);
        a.add_at_place(Block::MAX, 1);
        a.add_at_place(Block::MAX, 2);
        assert_eq!(a.to_hex_str(), "ffffffffffffffff ffffffffffffffff ffffffffffffffff");
        a.add(1);
        assert_eq!(a.to_hex_str(), "0000000000000001 0000000000000000 0000000000000000 0000000000000000");
        a.add_at_place(Block::MAX, 3);
        assert_eq!(a.to_hex_str(), "0000000000000001 0000000000000000 0000000000000000 0000000000000000 0000000000000000");
    }

    #[test]
    fn add() {
        // test very basic addition and vector grows
        let mut a = Accumulator::new();
        a.add(2);
        a.add(4);
        assert_eq!(a.to_hex_str(), "0000000000000006");
        a.add(Block::MAX);
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
        a.add_at_place(Block::MAX, 2);
        a.add_at_place(Block::MAX, 1);
        a.add_at_place(Block::MAX, 0);
        a.mul(Block::MAX);
        assert_eq!(a.to_hex_str(), "fffffffffffffffe ffffffffffffffff ffffffffffffffff 0000000000000001");
    }

    #[test]
    fn div() {
        let mut a = Accumulator::new();
        a.add(0xff);
        let r = a.div(2);
        assert_eq!(r, 1);
        assert_eq!(a.to_hex_str(), "000000000000007f");

        let mut a = Accumulator::new();
        let encode: [Block; 4] = [Block::MAX-10000, Block::MAX-1000, Block::MAX-100, Block::MAX-10];
        let mut decode: [Block; 4] = [0; 4];
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
        a.add(Block::MAX);
        a.mul(Block::MAX);
        a.div(Block::MAX);
        assert_eq!(a.to_hex_str(), "ffffffffffffffff");
    }
}
