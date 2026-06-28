use emulator::config::Config;
use emulator::emulator::Machine;

fn main() {
    let cfg = Config::load();
    let mut machine = Machine::new(cfg);
    machine.run();
}
