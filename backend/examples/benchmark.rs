use std::io;

use system76_keyboard_configurator_backend::Benchmark;

fn benchmark() -> io::Result<()> {
    let benchmark = Benchmark::new()?;
    for (port_desc, port_result) in benchmark.port_results.iter() {
        eprintln!("{}: {:.2?}", port_desc, port_result);
    }

    Ok(())
}

fn main() {
    benchmark().unwrap();
}
