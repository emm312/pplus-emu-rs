use crate::{cpu::addressable::Addressable, MAGIC_NUMBER};

pub struct IO;

impl Addressable for IO {
    fn load_file(&mut self, file: &std::path::Path) -> i32 {
        0
    }
    fn write(&mut self, loc: i32, val: i32) {
        match loc & 255 {
            0x00 => print!("{}", val & MAGIC_NUMBER),
            _ => (),
        }
    }
    fn read(&self, loc: i32) -> i32 {
        0
    }
}
