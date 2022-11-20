Waits for interrupts across a set of InputPins.

Blocks until an interrupt is raised and then returns a list of references to the
pin or pins that were the source. Note that it is possible that none of the
pins were actually the source of the error (though this is suspicious and will
cause a warning log) in which case the returned vector will be empty.

Each interrupt is represented by a tuple of a reference to the pin and the level
on the pin when the interrupt happened.

If the timeout expires the function will return [`None`].

# Example usage

```rust no_run
use rppal_pfd::{ChipSelect, HardwareAddress, InterruptMode, PiFaceDigital, SpiBus, SpiMode};
use std::time::Duration;

// Create an instance of the driver for the device with the hardware address
// (A1, A0) of 0b00 on SPI bus 0 clocked at 100kHz. The address bits are set using
// JP1 and JP2 on the PiFace Digital board.
let mut pfd = PiFaceDigital::new(
    HardwareAddress::new(0).expect("Invalid hardware address"),
    SpiBus::Spi0,
    ChipSelect::Cs0,
    100_000,
    SpiMode::Mode0,
)
.expect("Failed to create PiFace Digital");

// Creating interrupt pin on the fourth switch on the PiFace Digital card.
let mut interrupt_pin1 = pfd.get_pull_up_input_pin(3).expect("Bad pin");
interrupt_pin1.set_interrupt(InterruptMode::BothEdges).expect("Bad interrupt");

// Creating interrupt pin on the third switch on the PiFace Digital card.
let mut interrupt_pin2 = pfd.get_pull_up_input_pin(2).expect("Bad pin");
interrupt_pin2.set_interrupt(InterruptMode::BothEdges).expect("Bad interrupt");

loop {
    // Wait one minute for a button press...
    match pfd.poll_interrupts(
        &[&interrupt_pin1, &interrupt_pin2],
        false,
        Some(Duration::from_secs(60)),
    ) {
        Ok(Some(interrupts)) => {
            // Button(s) were pressed!
            for (i, (pin, level)) in interrupts.iter().enumerate() {
                let pin_no = pin.get_pin_number();
                println!("Interrupt[{i}]: pin({pin_no}) is {level}");
            }
        }

        Ok(None) => {
            println!("Poll timed out");
            break;
        }

        Err(e) => {
            eprintln!("Poll failed with {e}");
            break;
        }
    }
}
```

# Testing

Note that in testing environments or with the `mockspi` feature enabled, this
is replaced with a dummy interrupt poll function that always returns as if a
timeout occurred (even if specified timeout was "forever", which is frankly
wrong!)
