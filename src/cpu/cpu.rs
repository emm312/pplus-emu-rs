use std::path::Path;

use crate::io::IO;

use super::addressable::{Addressable, Memory};

const DOUBLE_WORD: [u32; 8] = [0x00004000, 0x00000000, 0xAAAAC00C, 0xA0000000, 0x00000000, 0x00000000, 0xFFFF0000, 0x0000F0F0];


pub struct CPU {
    primary_regfile: Vec<i32>,
    secondary_regfile: Vec<i32>,
    reg_ip: i32,
    reg_jp: i32,
    reg_rf: i32,
    pub reg_st: i32,
    skip: bool,
    mem: Memory,
    pub io_space: IO,
}

impl CPU {
    pub fn new() -> CPU {
        CPU {
            primary_regfile: vec![0; 16],
            secondary_regfile: vec![0; 16],
            reg_ip: 0,
            reg_jp: 0,
            reg_rf: 0,
            reg_st: 0,
            skip: false,
            mem: Memory::new(),
            io_space: IO::init(),
        }
    }

    pub fn load_prog(&mut self) {
        self.mem.load_file(Path::new("program.hex"));
    }

    fn get_imx(&mut self) -> i32 {
        let ip = self.reg_ip;
        self.reg_ip += 1;
        self.mem.read(ip)
    }

    pub fn exec(&mut self) {
        let instr_word = self.mem.read(self.reg_ip);
        self.reg_ip += 1;
        let opcode = get_opc(instr_word);
        if self.skip {
            if (DOUBLE_WORD[(opcode >> 5) as usize] & (1 << (opcode & 31))) != 0 {
                self.reg_ip += 1;
            } 
            self.skip = false;
        } else {
            self.exec_opcode(opcode, instr_word);
            self.primary_regfile[0] = 0;
        }
    }
    
    fn set_flags(&mut self, n: bool, v: bool, c: bool, z: bool) {
        let mut flags: i32 = self.reg_st & 4095;
        flags |= (z as i32) << 12;
        flags |= (c as i32) << 13;
        flags |= (v as i32) << 14;
        flags |= (n as i32) << 15;
        self.reg_st = flags;
   }

    fn eval_cond(&self, opcode: i32) -> bool {
        match opcode & 7 {
            0 => return is_set(self.reg_st, 13),
            1 => return is_set(self.reg_st, 14),
            2 => return is_set(self.reg_st, 12),
            3 => return !is_set(self.reg_st, 12),
            4 => return !is_set(self.reg_st, 13),
            5 => return !is_set(self.reg_st, 13) | is_set(self.reg_st, 12),
            6 => return (is_set(self.reg_st, 15) != is_set(self.reg_st, 14)) || is_set(self.reg_st, 12),
            _ => return false
        }
    }
    
    fn eval_prop(&self, opcode: i32, regval: i32) -> bool {
        match opcode & 7 {
            0 => return regval == 0,
            1 => return regval == self.reg_rf,
            2 => return (regval&32768) != 0,
            3 => return (regval&1) != 0,
            4 => return regval != 0,
            5 => return regval != self.reg_rf,
            6 => return (regval&32768) == 0,
            7 => return (regval&1) == 0,
            _ => return false
        }
    }

    fn exec_opcode(&mut self, opcode: i32, iw: i32) {
        //println!("[INFO] executing instruction {}", iw);
        match opcode {
            0 => self.reg_st ^= 1 << get_ims(iw), // sig
            1 => self.primary_regfile[get_dst(iw)] = self.primary_regfile[get_src(iw)], // movxx
            2 => self.secondary_regfile[get_dst(iw)] = self.primary_regfile[get_src(iw)], // movyx
            3 => self.primary_regfile[get_dst(iw)] = self.secondary_regfile[get_src(iw)], // movxy
            4 => self.secondary_regfile[get_dst(iw)] = self.secondary_regfile[get_src(iw)], // movyy
            5 => self.reg_st = self.primary_regfile[get_src(iw)], // lst
            6 => self.primary_regfile[get_dst(iw)] = self.reg_st, // sst
            7 => self.reg_rf = self.primary_regfile[get_src(iw)], // lrf
            8 => self.primary_regfile[get_dst(iw)] = self.reg_rf, // srf
            9 => self.reg_jp = self.primary_regfile[get_src(iw)], // ljp
            10 => self.primary_regfile[get_dst(iw)] = self.reg_jp, // sjp
            11 => self.reg_ip = self.reg_jp, // lip
            12 => self.primary_regfile[get_dst(iw)] = self.reg_ip+get_ims(iw), // sip
            13 => self.reg_ip += sxt8(get_iml(iw))-1, // jmpo
            14 => { // jnl
                self.primary_regfile[get_dst(iw)] = self.reg_ip+1;
                self.reg_ip = self.primary_regfile[get_src(iw)]+self.get_imx();
            },
            15 => self.skip = !is_set(self.reg_rf, get_ims(iw)), // prdr
            16..=23 => self.skip = !self.eval_cond(opcode), // prdc
            24..=31 => self.skip = !self.eval_prop(opcode, self.primary_regfile[get_dst(iw)]), // prdp

            32..=39 => self.reg_rf &= !(if self.eval_cond(opcode) { 0 } else { 1 << get_ims(iw) }), // rbcc
            40..=47 => self.reg_rf &= !(if self.eval_prop(opcode, self.primary_regfile[get_dst(iw)]) { 0 } else { 1 << get_ims(iw) }), // rbcp

            48..=55 => self.reg_rf |= if self.eval_cond(opcode) { 1 << get_ims(iw) } else { 0 }, // rbdc
            56..=63 => self.reg_rf |= if self.eval_prop(opcode, self.primary_regfile[get_dst(iw)]) { 1 << get_ims(iw) } else { 0 }, //rbdp
            64 => { // addrx
                let a = self.primary_regfile[get_dst(iw)];
                let b = self.primary_regfile[get_src(iw)];
                let sum = a + b;
                self.set_flags(is_neg(sum), is_ovf(a, b, sum), (sum&65536) != 0, is_zero(sum));
                self.primary_regfile[get_dst(iw)] = sum & 65535;
            }
            65 => { // addry
                let a = self.secondary_regfile[get_dst(iw)];
                let b = self.primary_regfile[get_src(iw)];
                let sum = a + b;
                self.set_flags(is_neg(sum), is_ovf(a, b, sum), (sum&65536) != 0, is_zero(sum));
                self.secondary_regfile[get_dst(iw)] = sum & 65535;
            }
            66 => { // addix
                let a = self.primary_regfile[get_dst(iw)];
                let b = self.get_imx();
                let sum = a + b;
                self.set_flags(is_neg(sum), is_ovf(a, b, sum), (sum&65536) != 0, is_zero(sum));
                self.primary_regfile[get_dst(iw)] = sum & 65535;
            }
            67 => { // addiy
                let a = self.primary_regfile[get_dst(iw)];
                let b = self.get_imx();
                let sum = a + b;
                self.set_flags(is_neg(sum), is_ovf(a, b, sum), (sum&65536) != 0, is_zero(sum));
                self.secondary_regfile[get_dst(iw)] = sum & 65535;
            }
            68 => { // addsx
                let a = self.primary_regfile[get_dst(iw)];
                let b = get_ims(iw);
                let sum = a + b + 1;
                self.set_flags(is_neg(sum), is_ovf(a, b, sum), (sum&65536) != 0, is_zero(sum));
                self.primary_regfile[get_dst(iw)] = sum & 65535;
            }
            69 => { // addsy
                let a = self.secondary_regfile[get_dst(iw)];
                let b = get_ims(iw);
                let sum = a + b + 1;
                self.set_flags(is_neg(sum), is_ovf(a, b, sum), (sum&65536) != 0, is_zero(sum));
                self.secondary_regfile[get_dst(iw)] = sum & 65535;
            }
            70 => { // addc
                let a = self.primary_regfile[get_dst(iw)];
                let b = self.primary_regfile[get_src(iw)];
                let sum = a + b + (is_set(self.reg_st, 13) as i32);
                self.set_flags(is_neg(sum), is_ovf(a, b, sum), (sum&65536) != 0, is_zero(sum));
                self.primary_regfile[get_dst(iw)] = sum & 65535;
            }
            71 => { // subrx
                let a = self.primary_regfile[get_dst(iw)];
                let b = self.primary_regfile[get_src(iw)];
                let sum = a - b;
                self.set_flags(is_neg(sum), is_ovf(a, b, sum), (sum&65536) != 0, is_zero(sum));
                self.primary_regfile[get_dst(iw)] = sum & 65535;
            }
            72 => { // subry
                let a = self.secondary_regfile[get_dst(iw)];
                let b = self.primary_regfile[get_src(iw)];
                let sum = a - b;
                self.set_flags(is_neg(sum), is_ovf(a, b, sum), (sum&65536) != 0, is_zero(sum));
                self.secondary_regfile[get_dst(iw)] = sum & 65535;
            }
            73 => { // subsx
                let a = self.primary_regfile[get_dst(iw)];
                let b = get_ims(iw);
                let sum = a - b;
                self.set_flags(is_neg(sum), is_ovf(a, b, sum), (sum&65536) != 0, is_zero(sum));
                self.primary_regfile[get_dst(iw)] = sum & 65535;
            }
            74 => { // subsy
                let a = self.secondary_regfile[get_dst(iw)];
                let b = get_ims(iw);
                let sum = a - b;
                self.set_flags(is_neg(sum), is_ovf(a, b, sum), (sum&65536) != 0, is_zero(sum));
                self.secondary_regfile[get_dst(iw)] = sum & 65535;
            }
            75 => { // subc
                let a = self.primary_regfile[get_dst(iw)];
                let b = self.primary_regfile[get_src(iw)];
                let sum = a - b + (if is_set(self.reg_st, 13) { 0 } else { -1 });
                self.set_flags(is_neg(sum), is_ovf(a, b, sum), (sum&65536) != 0, is_zero(sum));
                self.primary_regfile[get_dst(iw)] = sum & 65535;
            }
            76 => { // cmpx
                let a = self.primary_regfile[get_dst(iw)];
                let b = self.primary_regfile[get_src(iw)];
                let sum = a - b;
                self.set_flags(is_neg(sum), is_ovf(a, b, sum), (sum&65536) != 0, is_zero(sum));
            }
            77 => { // cmpy
                let a = self.secondary_regfile[get_dst(iw)];
                let b = self.secondary_regfile[get_src(iw)];
                let sum = a - b;
                self.set_flags(is_neg(sum), is_ovf(a, b, sum), (sum&65536) != 0, is_zero(sum));
            }
            78 => { // pen
                let rev: [i32; 16] = [0,8,4,12,2,10,6,14,1,9,5,13,3,11,7,15];
                let src = self.primary_regfile[get_src(iw)];
                let mut imm = self.get_imx();
                let mut dest = 0;
                for _ in 0..4 {
                    let op = imm&15;
                    imm >>= 4;
                    let shift = (op&3)<<2;
                    let mut nibble = (src & 15 << shift) >> shift;
                    match op >> 2 {
                        1 => nibble ^= 15,
                        2 => nibble = rev[nibble as usize],
                        3 => nibble = if op&1 != 0 { 15 } else { 0 },
                        _ => unreachable!("[ERROR] pen instr err")
                    }
                    nibble <<= 12;
                    dest = dest >> 4 |nibble;
                }
                self.set_flags(is_neg(dest), false, false, is_zero(dest));
                self.primary_regfile[get_dst(iw)] = dest;
            }
            79 => { // peb
                let mut imm = self.get_imx();
                let dst_idx = (imm >> 10) & 12;
                let src_idx = (imm >> 12) & 12;
                let mut src = self.primary_regfile[get_src(iw)];
                let mut dst = src;
                src = src >> src_idx & 15;
                let mut nibble = 0;
                for _ in 0..4 {
                    let op = imm & 7;
                    imm >>= 3;
                    let shift = op & 3;
                    let mut bit = (src & 1 << shift) >> shift;
                    bit ^= op >> 2;
                    bit <<= 3;
                    nibble = nibble >> 1 | bit;
                }
                dst &= !(15<<dst_idx);
                dst |= nibble<<dst_idx;
                self.set_flags(is_neg(dst), false, false, is_zero(dst));
                self.primary_regfile[get_dst(iw)] = dst;
            }
            80 => { // mulr
                let a = self.primary_regfile[get_dst(iw)];
                let b = self.primary_regfile[get_src(iw)];
                let prod = a*b;
                self.set_flags(is_neg(prod), (prod as u32 >> 16) != 0, false, is_zero(prod));
                self.primary_regfile[get_dst(iw)] = prod&65535;
            }
            81 => { // muli
                let a = self.primary_regfile[get_src(iw)];
                let b = self.get_imx();
                let prod = a*b;
                self.set_flags(is_neg(prod), (prod as u32 >> 16) != 0, false, is_zero(prod));
                self.primary_regfile[get_dst(iw)] = prod&65535;
            }
            82 => { // umlr
                let a = self.primary_regfile[get_dst(iw)];
                let b = self.primary_regfile[get_src(iw)];
                let prod = ((a*b) as u32 >> 16) as i32;
                self.set_flags(is_neg(prod), false, false, is_zero(prod));
                self.primary_regfile[get_dst(iw)] = prod;
            }
            83 => { // umli
                let a = self.primary_regfile[get_src(iw)];
                let b = self.get_imx();
                let prod = ((a*b) as u32 >> 16) as i32;
                self.set_flags(is_neg(prod), false, false, is_zero(prod));
                self.primary_regfile[get_dst(iw)] = prod;
            }
            84 => { // smlr
                let mut a = self.primary_regfile[get_dst(iw)];
                let mut b = self.primary_regfile[get_src(iw)];
                a |= if is_neg(a) { -65536 } else { 0 };
                b |= if is_neg(a) { -65536i32 } else { 0 };
                let prod = ((a*b) as u32 >> 16) as i32;
                self.set_flags(is_neg(prod), false, false, is_zero(prod));
                self.primary_regfile[get_dst(iw)] = prod;
            }
            85 => { // smli
                let mut a = self.primary_regfile[get_src(iw)];
                let mut b = self.get_imx();
                a |= if is_neg(a) { -65536 } else { 0 };
                b |= if is_neg(a) { -65536 } else { 0 };
                let prod = ((a*b) as u32 >> 16) as i32;
                self.set_flags(is_neg(prod), false, false, is_zero(prod));
                self.primary_regfile[get_dst(iw)] = prod;
            }
            86 => { // andr
                let a = self.primary_regfile[get_dst(iw)];
                let b = self.primary_regfile[get_src(iw)];
                let res = a & b;
                self.set_flags(is_neg(res), false, false, is_zero(res));
                self.primary_regfile[get_dst(iw)] = res;
            }
            87 => { // andi
                let a = self.primary_regfile[get_src(iw)];
                let b = self.get_imx();
                let res = a & b;
                self.set_flags(is_neg(res), false, false, is_zero(res));
                self.primary_regfile[get_dst(iw)] = res;
            }
            88 => { // nndr
                let a = self.primary_regfile[get_dst(iw)];
                let b = self.primary_regfile[get_src(iw)];
                let res = !(a & b);
                self.set_flags(is_neg(res), false, false, is_zero(res));
                self.primary_regfile[get_dst(iw)] = res;
            }
            89 => { // nndi
                let a = self.primary_regfile[get_src(iw)];
                let b = self.get_imx();
                let res = !(a & b);
                self.set_flags(is_neg(res), false, false, is_zero(res));
                self.primary_regfile[get_dst(iw)] = res;
            }
            90 => { // iorr
                let a = self.primary_regfile[get_dst(iw)];
                let b = self.primary_regfile[get_src(iw)];
                let res = a | b;
                self.set_flags(is_neg(res), false, false, is_zero(res));
                self.primary_regfile[get_dst(iw)] = res;
            }
            91 => { // iori
                let a = self.primary_regfile[get_src(iw)];
                let b = self.get_imx();
                let res = a | b;
                self.set_flags(is_neg(res), false, false, is_zero(res));
                self.primary_regfile[get_dst(iw)] = res;
            }
            92 => { // norr
                let a = self.primary_regfile[get_dst(iw)];
                let b = self.primary_regfile[get_src(iw)];
                let res = !(a | b);
                self.set_flags(is_neg(res), false, false, is_zero(res));
                self.primary_regfile[get_dst(iw)] = res;
            }
            93 => { // nori
                let a = self.primary_regfile[get_src(iw)];
                let b = self.get_imx();
                let res = !(a | b);
                self.set_flags(is_neg(res), false, false, is_zero(res));
                self.primary_regfile[get_dst(iw)] = res;
            }
            94 => { // xorr
                let a = self.primary_regfile[get_dst(iw)];
                let b = self.primary_regfile[get_src(iw)];
                let res = a ^ b;
                self.set_flags(is_neg(res), false, false, is_zero(res));
                self.primary_regfile[get_dst(iw)] = res;
            }
            95 => { // xori
                let a = self.primary_regfile[get_src(iw)];
                let b = self.get_imx();
                let res = a ^ b;
                self.set_flags(is_neg(res), false, false, is_zero(res));
                self.primary_regfile[get_dst(iw)] = res;
            }
            96 => self.set_flags(false, false, (self.primary_regfile[get_dst(iw)] & 1 << (self.primary_regfile[get_src(iw)]&15)) != 0, false), // bxtr
            97 => self.set_flags(false, false, (self.primary_regfile[get_dst(iw)] & 1 << get_ims(iw)) != 0, false), // bxts
            98 => { // bdpr
                let mut val = self.primary_regfile[get_dst(iw)];
                let pos = self.primary_regfile[get_src(iw)]&15;
                val &= !(1<<pos);
                val |= if is_set(self.reg_st, 13) { 1 << pos } else { 0 };
                self.primary_regfile[get_dst(iw)] = val;
            }
            99 => { // bdps
                let mut val = self.primary_regfile[get_dst(iw)];
                let pos = get_ims(iw);
                val &= !(1<<pos);
                val |= if is_set(self.reg_st, 13) { 1 << pos } else { 0 };
                self.primary_regfile[get_dst(iw)] = val;
            }
            100 => self.primary_regfile[get_dst(iw)] ^= 1 << (self.primary_regfile[get_src(iw)] & 15), // bngr
            101 => self.primary_regfile[get_dst(iw)] ^= 1 << get_ims(iw), // bngs
            102 => self.set_flags(false, false, (self.reg_rf & 1 << (self.primary_regfile[get_src(iw)]&15)) != 0, false), // rxtr
            103 => self.set_flags(false, false, (self.reg_rf & 1 << get_ims(iw)) != 0, false), // rxts
            104 => { // rdpr
                let mut val = self.reg_rf;
                let pos = self.primary_regfile[get_src(iw)] & 15;
                val &= !(1 << pos);
                val |= if is_set(self.reg_st, 13) { 1 << pos } else { 0 };
                self.reg_rf = val;
            }
            105 => { // rdps
                let mut val = self.reg_rf;
                let pos = get_ims(iw);
                val &= !(1 << pos);
                val |= if is_set(self.reg_st, 13) { 1 << pos } else { 0 };
                self.reg_rf = val;
            }
            106 => self.primary_regfile[get_dst(iw)] = if self.reg_rf & 1 << (self.primary_regfile[get_src(iw) & 15]) != 0 { 0xffff } else { 0 }, // rbrr
            107 => self.primary_regfile[get_dst(iw)] = if self.reg_rf & 1 << get_ims(iw) != 0 { 0xffff } else { 0 }, // rbrs
            108 => { // asr
                let val = (self.primary_regfile[get_src(iw)]<<16)>>16;
                let res = val >> 1 & 65535;
                self.primary_regfile[get_dst(iw)] = res;
                self.set_flags(is_neg(res), false, val&1 != 0, is_zero(res));
            }
            110 => { // abrr
                let val = (self.primary_regfile[get_dst(iw)]<<16)>>16;
                let res = val >> (self.primary_regfile[get_src(iw)]&15)&65535;
                self.primary_regfile[get_dst(iw)] = res;
                self.set_flags(is_neg(res), false, false, is_zero(res));
            }
            111 => { // abrs
                let val = (self.primary_regfile[get_dst(iw)]<<16)>>16;
                let res = val >> get_ims(iw)&65535;
                self.primary_regfile[get_dst(iw)] = res;
                self.set_flags(is_neg(res), false, false, is_zero(res));
            }
            112 => { // lsr
                let val = self.primary_regfile[get_src(iw)];
                let res = val >> 1;
                self.primary_regfile[get_dst(iw)] = res;
                self.set_flags(is_neg(res), false, val & 1 != 0, is_zero(res));
            }
            113 => { // lcr
                let val = self.primary_regfile[get_src(iw)];
                let res = val >> 1 | if is_set(self.reg_st, 13) { 1 << 15 } else { 0 };
                self.primary_regfile[get_dst(iw)] = res;
                self.set_flags(is_neg(res), false, (val & 1) != 0, is_zero(iw));
            }
            114 => { // lbrr
                let val = self.primary_regfile[get_dst(iw)];
                let res = val >> (self.primary_regfile[get_src(iw)]&15);
                self.primary_regfile[get_dst(iw)] = res;
                self.set_flags(is_neg(res), false, false, is_zero(res));
            }
            115 => { // lbrs
                let val = self.primary_regfile[get_dst(iw)];
                let res = val >> get_ims(iw);
                self.primary_regfile[get_dst(iw)] = res;
                self.set_flags(is_neg(res), false, false, is_zero(res));
            }
            116 => { // lsl
                let val = self.primary_regfile[get_src(iw)] << 1;
                self.primary_regfile[get_dst(iw)] = val & 65535;
                self.set_flags(is_neg(val), false, (val & 65535) != 0, is_zero(val));
            }
            117 => { // lcl
                let val = self.primary_regfile[get_src(iw)]<<1 | if is_set(self.reg_st, 13) { 1 } else { 0 };
                self.primary_regfile[get_dst(iw)] = val & 65535;
                self.set_flags(is_neg(val), false, val & 65535 != 0, is_zero(val));
            }
            118 => { // lblr
                let val = self.primary_regfile[get_dst(iw)]<<(self.primary_regfile[get_src(iw)]&15);
                self.primary_regfile[get_dst(iw)] = val & 65535;
                self.set_flags(is_neg(val), false, false, is_zero(val));
            }
            119 => { // lbls
                let val = self.primary_regfile[get_dst(iw)]<<get_ims(iw);
                self.primary_regfile[get_dst(iw)] = val & 65535;
                self.set_flags(is_neg(val), false, false, is_zero(val));
            }
            120 => { // rbm
                self.reg_rf &= !(1<<get_dst(iw));
                self.reg_rf |= if is_set(self.reg_rf, get_src(iw) as i32) { 1 << get_dst(iw) } else { 0 };
            }
            121 => { // rbn
                self.reg_rf &= !(1<<get_dst(iw));
                self.reg_rf |= if is_set(self.reg_rf, get_src(iw) as i32) { 0 } else { 1 << get_dst(iw) };
            }
            122 => self.reg_rf &= !if is_set(self.reg_rf, get_src(iw) as i32) { 0 } else { 1 << get_dst(iw) as i32 }, // rbc
            123 => self.reg_rf |= !if is_set(self.reg_rf, get_src(iw) as i32) { 1 << get_dst(iw) as i32 } else { 0 }, // rbd
            124 => self.primary_regfile[get_dst(iw)] = self.mem.read(self.primary_regfile[get_src(iw)]), // ldrx
            125 => { // ldix
                let val = self.get_imx();
                self.primary_regfile[get_dst(iw)] = self.mem.read(self.primary_regfile[get_src(iw)] + val); 
            },
            126 => self.mem.write(self.primary_regfile[get_src(iw)], self.primary_regfile[get_dst(iw)]), // strx
            127 => { // stix
                let val = self.get_imx();
                self.mem.write(self.primary_regfile[get_src(iw)] + val, self.primary_regfile[get_dst(iw)]);
            }
            
            128..=143 => self.primary_regfile[get_dst(iw)] = sxt8(get_imh(iw)), // lsi
            144..=159 => { // lui
                let mut val = self.primary_regfile[get_dst(iw)] & 255;
                val |= get_imh(iw) << 8;
                self.primary_regfile[get_dst(iw)] = val;
            }
            160..=175 => self.primary_regfile[get_dst(iw)] = self.io_space.read(get_imh(iw)), // inp
            176..=191 => self.io_space.write(get_imh(iw), self.primary_regfile[get_dst(iw)]), // out
            192..=199 => if self.eval_cond(opcode) { self.reg_ip = self.primary_regfile[get_src(iw)]; }, // brcr
            200..=207 => if self.eval_prop(opcode, self.primary_regfile[get_dst(iw)]) { self.reg_ip = self.primary_regfile[get_src(iw)]; }, // brpr
            208..=215 => { // brci
                let imm = self.get_imx();
                if self.eval_cond(opcode) {
                    self.reg_ip = self.primary_regfile[get_src(iw)] + imm;
                }
            }
            216..=223 => { // brpi
                let imm = self.get_imx();
                if self.eval_prop(opcode, self.primary_regfile[get_dst(iw)]) {
                    self.reg_ip = self.primary_regfile[get_src(iw)] + imm;
                }
            }
            224 => self.primary_regfile[get_dst(iw)] = self.mem.read(self.secondary_regfile[get_src(iw)]), // ldry
            225 => self.primary_regfile[get_dst(iw)] = self.mem.read({ self.secondary_regfile[get_src(iw)] -= 1; self.secondary_regfile[get_src(iw)] }), // mldry
            226 => self.primary_regfile[get_dst(iw)] = self.mem.read({ self.secondary_regfile[get_src(iw)] += 1; self.secondary_regfile[get_src(iw)]-1 }), // ldryp
            227 => self.primary_regfile[get_dst(iw)] = self.mem.read({ self.secondary_regfile[get_src(iw)] += 1; self.secondary_regfile[get_src(iw)]}), // pldry
            228 => { // ldiy
                let imm = self.get_imx();
                self.primary_regfile[get_dst(iw)] = self.mem.read(self.secondary_regfile[get_src(iw)] + imm);
            }
            229 => { // mldiy
                let imm = self.get_imx();
                self.primary_regfile[get_dst(iw)] = self.mem.read({ self.secondary_regfile[get_src(iw)] -= 1; self.secondary_regfile[get_src(iw)] } + imm);
            }
            230 => { // ldiyp
                let imm = self.get_imx();
                self.primary_regfile[get_dst(iw)] = self.mem.read(self.secondary_regfile[get_src(iw)] + imm);
                self.secondary_regfile[get_src(iw)] += 1;
            }
            231 => { // pldiy
                let imm = self.get_imx();
                self.primary_regfile[get_dst(iw)] = self.mem.read({ self.secondary_regfile[get_src(iw)] += 1; self.secondary_regfile[get_src(iw)] + imm });
            }
            232 => { // stry
                self.mem.write(self.secondary_regfile[get_src(iw)], self.primary_regfile[get_dst(iw)]);
            }
            233 => { // mstry
                self.mem.write({ self.secondary_regfile[get_src(iw)] -= 1; self.secondary_regfile[get_src(iw)] }, self.primary_regfile[get_dst(iw)]);
            }
            234 => {  // stryp
                self.mem.write({ self.secondary_regfile[get_src(iw)] += 1; self.secondary_regfile[get_src(iw)]-1 }, self.primary_regfile[get_dst(iw)]);
            }
            235 => { // pstry
                self.mem.write({ self.secondary_regfile[get_src(iw)] += 1; self.secondary_regfile[get_src(iw)]}, self.primary_regfile[get_dst(iw)]);
            }
            236 => { // stiy
                let imm = self.get_imx();
                self.mem.write(self.secondary_regfile[get_src(iw)] + imm, self.primary_regfile[get_dst(iw)]);
            }
            237 => { // mstiy
                let imm = self.get_imx();
                self.mem.write({ self.secondary_regfile[get_src(iw)] -= 1; self.secondary_regfile[get_src(iw)] } + imm, self.primary_regfile[get_dst(iw)]);
            }
            238 => { // stiyp
                let imm = self.get_imx();
                self.mem.write(self.secondary_regfile[get_src(iw)] + imm, self.primary_regfile[get_dst(iw)]);
                self.secondary_regfile[get_src(iw)] += 1;
            }
            239 => { // pstiy
                let imm = self.get_imx();
                self.mem.write({ self.secondary_regfile[get_src(iw)] += 1; self.secondary_regfile[get_src(iw)] + imm }, self.primary_regfile[get_dst(iw)]);
            }
            240..=247 => if self.eval_cond(opcode) { self.reg_ip += sxt8(get_iml(iw))-1 },
            _ => panic!("[ERR] Invalid instruction: {}; {}", iw, opcode),
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

pub fn is_set(num: i32, pos: i32) -> bool {
    ((num & 65535) & (1<<(pos&15))) != 0
}

fn is_neg(val: i32) -> bool {
    (val&32768) != 0
}

fn is_ovf(a: i32, b: i32, sum: i32) -> bool {
    (is_neg(a) != is_neg(sum)) && (is_neg(a) == is_neg(b))
}

fn is_zero(val: i32) -> bool {
    (val&65535) == 0
}