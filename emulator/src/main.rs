#![allow(dead_code)]

mod acia;
mod bus;
mod config;
mod cpu;
mod disk;
mod emulator;
mod mmu;
mod peripherals;
mod rom;
mod rtc;
mod xt_ide;

use config::Config;

fn main() {
    let cfg = Config::load();
    let mut machine = emulator::Machine::new(cfg);
    machine.run();
}
