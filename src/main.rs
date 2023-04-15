use std::{time::{Instant, Duration}, thread::{Thread, sleep}, os::unix::thread};

use pplus_emu::cpu::cpu::{CPU, is_set};

fn main() {
    let mut cpu = CPU::new();
    cpu.load_prog();
    let mut counter: u128 = 0;
    let time = Instant::now();
    let max_insts = 100_000_000;
    loop {
        let hlt = is_set(cpu.reg_st, 0) /* || counter >= max_insts */;
        if hlt {
            break;
        }
        cpu.exec();
        counter += 1;
    }
    let elapsed = time.elapsed();
    std::thread::sleep(Duration::from_millis(1000));
    print!("\n[INFO] Took {} ns to execute {} instructions, ", elapsed.as_nanos(), counter);
    println!(" ({} kHz)", (counter*1000000) as u128/elapsed.as_nanos())
}
