use crate::io::IO;

use super::addressable::{Addressable, Memory};

struct State {
    pub primary_regfile: Vec<i32>,
    pub secondary_regfile: Vec<i32>,
    pub reg_ip: i32,
    pub reg_jp: i32,
    pub reg_rf: i32,
    pub reg_st: i32,
    pub skip: bool,
}

impl State {
    pub fn new() -> State {
        State {
            primary_regfile: Vec::with_capacity(16),
            secondary_regfile: Vec::with_capacity(16),
            reg_ip: 0,
            reg_jp: 0,
            reg_rf: 0,
            reg_st: 0,
            skip: false,
        }
    }
}

lazy_static! {
    static ref DOUBLE_WORD: Vec<u32> = vec![0x00004000, 0x00000000, 0xAAAAC00C, 0xA0000000, 0x00000000, 0x00000000, 0xFFFF0000, 0x0000F0F0];
}

pub struct CPU {
    state: State,
    mem: Memory,
    io_space: IO,
}

impl CPU {
    pub fn new() -> CPU {
        CPU {
            state: State::new(),
            mem: Memory::new(),
            io_space: IO,
        }
    }

    fn get_imx(&mut self) -> i32 {
        let ip = self.state.reg_ip;
        self.state.reg_ip += 1;
        self.mem.read(ip)
    }

    pub fn exec(&mut self) {
        let instr_word = self.mem.read(self.state.reg_ip);
        self.state.reg_ip += 1;
        let opcode = get_opc(instr_word);
        if self.state.skip {
            if (DOUBLE_WORD[(opcode >> 5) as usize] & (1 << (opcode & 31))) != 0 {
                self.state.reg_ip += 1;
                self.state.skip = false;
            } else {
                self.exec_opcode(opcode, instr_word);
                self.state.primary_regfile[0] = 0;
            }
        }
    }

    fn eval_cond(&self, opcode: i32) -> bool {
        match opcode & 7 {
            0 => return is_set(self.state.reg_st, 13),
            1 => return is_set(self.state.reg_st, 14),
            2 => return is_set(self.state.reg_st, 12),
            3 => return !is_set(self.state.reg_st, 12),
            4 => return !is_set(self.state.reg_st, 13),
            5 => return !is_set(self.state.reg_st, 13) | is_set(self.state.reg_st, 12),
            6 => return (is_set(self.state.reg_st, 15) != is_set(self.state.reg_st, 14)) || is_set(self.state.reg_st, 12),
            _ => return false
        }
    }
    
    fn eval_prop(&self, opcode: i32, regval: i32) -> bool {
        match opcode & 7 {
            0 => return regval == 0,
            1 => return regval == self.state.reg_rf,
            2 => return (regval&32768) != 0,
            3 => return (regval&1) != 0,
            4 => return regval != 0,
            5 => return regval != self.state.reg_rf,
            6 => return (regval&32768) == 0,
            7 => return (regval&1) == 0,
            _ => return false
        }
    }

    fn exec_opcode(&mut self, opcode: i32, iw: i32) {
        match opcode {
            0 => self.state.reg_st ^= 1 << get_ims(iw), // sig
            1 => self.state.primary_regfile[get_dst(iw)] = self.state.primary_regfile[get_src(iw)], // movxx
            2 => self.state.secondary_regfile[get_dst(iw)] = self.state.primary_regfile[get_src(iw)], // movyx
            3 => self.state.primary_regfile[get_dst(iw)] = self.state.secondary_regfile[get_src(iw)], // movxy
            4 => self.state.secondary_regfile[get_dst(iw)] = self.state.secondary_regfile[get_src(iw)], // movyy
            5 => self.state.reg_st = self.state.primary_regfile[get_src(iw)], // lst
            6 => self.state.primary_regfile[get_dst(iw)] = self.state.reg_st, // sst
            7 => self.state.reg_rf = self.state.primary_regfile[get_src(iw)], // lrf
            8 => self.state.primary_regfile[get_dst(iw)] = self.state.reg_rf, // srf
            9 => self.state.reg_jp = self.state.primary_regfile[get_src(iw)], // ljp
            10 => self.state.primary_regfile[get_dst(iw)] = self.state.reg_jp, // sjp
            11 => self.state.reg_ip = self.state.reg_jp, // lip
            12 => self.state.primary_regfile[get_dst(iw)] = self.state.reg_ip+get_ims(iw), // sip
            13 => self.state.reg_ip += sxt8(get_iml(iw))-1, // jmpo
            14 => { // jnl
                self.state.primary_regfile[get_dst(iw)] = self.state.reg_ip+1;
                self.state.reg_ip = self.state.primary_regfile[get_src(iw)]+self.get_imx();
            },
            15 => self.state.skip = !is_set(self.state.reg_rf, get_ims(iw)), // prdr
            16..=23 => self.state.skip = !self.eval_cond(opcode), // prdc
            24..=31 => self.state.skip = !self.eval_prop(opcode, self.state.primary_regfile[get_dst(iw)]), // prdp

            32..=39 => self.state.reg_rf &= !(if self.eval_cond(opcode) { 0 } else { 1 << get_ims(iw) }), // rbcc
            40..=47 => self.state.reg_rf &= !(if self.eval_prop(opcode, self.state.primary_regfile[get_dst(iw)]) { 0 } else { 1 << get_ims(iw) }), // rbcp

            48..=55 => self.state.reg_rf |= if self.eval_cond(opcode) { 1 << get_ims(iw) } else { 0 }, // rbdc
            56..=63 => self.state.reg_rf |= if self.eval_prop(opcode, self.state.primary_regfile[get_dst(iw)]) { 1 << get_ims(iw) } else { 0 }, //rbdp
            // n till 127
            _ => panic!("[ERR] Invalid instruction: {}", iw),
        }
    }
}

fn get_opc(word: i32) -> i32 {
    word >> 8 & 255
}

fn get_src(word: i32) -> usize {
    (word >> 4 & 15) as usize
}

fn get_dst(word: i32) -> usize {
    (word & 15) as usize
}

fn get_imh(word: i32) -> i32 {
    word >> 4 & 255
}

fn get_iml(word: i32) -> i32 {
    word & 255
}

fn get_ims(word: i32) -> i32 {
    word >> 4 & 15
}

fn sxt8(val: i32) -> i32 {
    if val <= 127 {
        val
    } else {
        0xFF00 | val
    }
}

fn is_set(num: i32, pos: i32) -> bool {
    ((num & 65535) & (1<<(pos&15))) != 0
}