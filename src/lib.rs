pub mod cpu;
pub mod io;

pub const MAGIC_NUMBER: i32 = u16::MAX as i32;

#[macro_use]
extern crate lazy_static;
