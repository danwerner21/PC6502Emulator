fn main() {
    let cfg = emulator::Config::load();
    let mut machine = emulator::Machine::new(cfg);
    machine.run();
}
