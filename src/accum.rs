//! Accumulator representing a very large unsigned number. Starts at zero and 
//! can be updated by adding, multiplying and dividing by unsigned values

use std::vec::Vec;
use std::string::String;
use std::mem::transmute;

#[derive(Debug)]
pub struct Accum {
    data: Vec<u32>,
}

impl Accum {

    pub fn new() -> Self {
        Self {data: Vec::new()}
    }

    fn add_at_place(&mut self, value: u32, place: usize) {
        let mut carry: u64 = value as u64;
        let mut vec_index: usize = place;
        // add digits until the accumulator is big enough 
        while vec_index > self.data.len() {
            self.data.push(0);
        }
        // Keep adding and carrying until the carry value is 0
        while carry != 0 {
            if vec_index == self.data.len() {
                self.data.push(carry as u32);
                carry = 0;
            } else {
                let result: u64 = carry + self.data[vec_index] as u64;
                let parts: [u32; 2] = unsafe { transmute(result) };
                self.data[vec_index] = parts[0];
                carry = parts[1] as u64;
                vec_index += 1;
            }
        }
    }

    pub fn add(&mut self, value: u32) {
        self.add_at_place(value, 0);
    }

    pub fn mult(&mut self, value: u32) {
    }

    pub fn to_hex_str(&self) -> String {
        let mut s: String = String::from("");
        for digit in self.data.iter().rev() {
            let digit_str: String = format!("{:08x} ", digit);
            s.push_str(&digit_str);
        }
        s.pop();
        s
    }
}

#[cfg(test)]
mod tests {

    use crate::accum::Accum;

    #[test]
    fn test_accum_add_at_place() {
        let mut a = Accum::new();
        a.add_at_place(3, 2);
        assert_eq!(a.to_hex_str(), "00000003 00000000 00000000");
        a.add_at_place(2, 1);
        assert_eq!(a.to_hex_str(), "00000003 00000002 00000000");
        a.add_at_place(1, 0);
        assert_eq!(a.to_hex_str(), "00000003 00000002 00000001");
        a.add_at_place(u32::MAX, 1);
        assert_eq!(a.to_hex_str(), "00000004 00000001 00000001");

        let mut a2 = Accum::new();
        a2.add_at_place(u32::MAX, 0);
        a2.add_at_place(u32::MAX, 1);
        a2.add_at_place(u32::MAX, 2);
        assert_eq!(a2.to_hex_str(), "ffffffff ffffffff ffffffff");
        a2.add(1);
        assert_eq!(a2.to_hex_str(), "00000001 00000000 00000000 00000000");
        a2.add_at_place(u32::MAX, 3);
        assert_eq!(a2.to_hex_str(), "00000001 00000000 00000000 00000000 00000000");
    }

    #[test]
    fn test_accum_add() {
        let mut a = Accum::new();
        a.add(2);
        a.add(4);
        assert_eq!(a.to_hex_str(), "00000006");
        a.add(u32::MAX);
        assert_eq!(a.to_hex_str(), "00000001 00000005");
        a.add(0xf0);
        assert_eq!(a.to_hex_str(), "00000001 000000f5");
    }
}