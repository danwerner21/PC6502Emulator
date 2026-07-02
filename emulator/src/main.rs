fn main() {
    let cfg = emulator::Config::load();
    eprintln!("Config: disk_image = {:?}", cfg.disk_image);
    eprintln!("Config: rom_hex    = {:?}", cfg.rom_hex);
    let debug = std::env::args().any(|a| a == "--debug");
    let mut machine = emulator::Machine::new(cfg);
    machine.debug = debug;
    machine.run();
}
