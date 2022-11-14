// Check reliability of operation of the SPI bus at different speeds.
//
// Writes alternating data to an output pin and reads back the value to check that data
// was reliably written.
//
// Note that if you just compile this with something like:
//
//   cross build --release --example spi-speed-test --target arm-unknown-linux-gnueabihf
//
// You will find that the resulting executable is using the mock-SPI version of the
// rppal-mcp23s17 crate because the dev-dependencies in our Cargo.toml specify that in
// order to be able to run our unit tests and cargo compile examples with dev
// dependencies. Copy this code into a new project or temporarily comment out the
// dev-dependency on rppal-mcp23s17 and use the mainline dependency instead.

use std::time::Instant;

use anyhow::Result;
use log::info;
use rppal_pfd::{ChipSelect, HardwareAddress, Level, PiFaceDigital, SpiBus, SpiMode};

fn main() -> Result<()> {
    env_logger::init();

    info!("Speed test started!");

    for clock_speed in (100_000..5_000_001).step_by(100_000) {
        info!("Speed test started at {clock_speed} Hz!");

        let mut pfd = PiFaceDigital::new(
            HardwareAddress::new(0).unwrap(),
            SpiBus::Spi0,
            ChipSelect::Cs0,
            clock_speed,
            SpiMode::Mode0,
        )?;
        pfd.init()?;

        let output_pin = pfd.get_output_pin(2)?;
        let mut good_count = 0;
        let mut bad_count = 0;

        let start_time = Instant::now();
        for i in 0..1000 {
            let request_level: Level = ((i & 1) as u8).into();
            output_pin.write(request_level)?;
            if request_level == output_pin.read()? {
                good_count += 1
            } else {
                bad_count += 1
            }
        }
        let time_taken = Instant::now() - start_time;
        println!(
            "Speed: {clock_speed}  Good: {good_count} Bad: {bad_count}  Duration: {time_taken:?}"
        );
    }

    Ok(())
}
