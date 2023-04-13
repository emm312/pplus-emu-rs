use std::path::Path;

use crate::MAGIC_NUMBER;

pub trait Addressable {
    fn read(&self, loc: i32) -> i32;
    fn write(&mut self, loc: i32, val: i32);

    fn load_file(&mut self, file: &Path) -> i32;
}

pub struct Memory {
    memory: Vec<i32>,
}

impl Addressable for Memory {
    fn load_file(&mut self, file: &Path) -> i32 {
        let content = std::fs::read_to_string(file).unwrap();
        let mut lines = content.lines().collect::<Vec<&str>>();
        if lines[0] != "v2.0 raw" {
            panic!("[ERR] File is of invalid format.");
        }
        lines.remove(0);
        let words = lines
            .iter()
            .map(|elem| format!("{} ", elem))
            .collect::<Vec<String>>()
            .concat()
            .split_whitespace()
            .map(|e| {
                i32::from_str_radix(e, 16).unwrap_or_else(|_| {
                    println!("[WARN] Invalid Hex: {}, replacing with 0", e);
                    0
                })
            })
            .collect::<Vec<i32>>();

        for (pos, val) in words.iter().enumerate() {
            if val & MAGIC_NUMBER != *val {
                println!("[WARN] Value {} at location {} out of range, trimming to 16 bits.", val, pos);
            }
            self.write(pos as i32, val & MAGIC_NUMBER)
        }

        return (words.len() - 1) as i32;
    }

    fn read(&self, loc: i32) -> i32 {
        self.memory[(loc & MAGIC_NUMBER) as usize]
    }

    fn write(&mut self, loc: i32, val: i32) {
        self.memory[(loc & MAGIC_NUMBER) as usize] = val;
    }
}

impl Memory {
    pub fn new() -> Memory {
        Memory { memory: vec![0; 65536] }
    }
}
