use pplus_emu::cpu::cpu::CPU;

fn main() {
    let mut cpu = CPU::new();
    cpu.load_prog();
}
