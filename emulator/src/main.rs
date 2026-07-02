fn main() {
    let cfg = emulator::Config::load();
    let debug = std::env::args().any(|a| a == "--debug");
    let mut machine = emulator::Machine::new(cfg);
    machine.debug = debug;
    machine.run();
}
