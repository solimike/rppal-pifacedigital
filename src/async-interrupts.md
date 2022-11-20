Waits for interrupts across a set of InputPins in a separate thread

Spawns a separate thread that then blocks until an interrupt is raised and then calls
the supplied callback.

The provided callback is called with a single parameter `level` that represents the
level on the _Raspberry Pi's GPIO_ input pin (not the MCP23S17). As the interrupts are
active-low, this will always be `GpioLevel::Low` for the real code. The parameter is
replaced with a dummy `bool` when the **mockspi** feature is enabled as the GPIO code
cannot be guaranteed to compile in non-target environments so the feature-flag causes it
to be skipped. Since this parameter is unlikely to be of interest, it will normally be
an anonymous placeholder (_i.e._ `|_|`) and so will compile equally well with or without
the **mockspi** feature.

Note that a potential to deadlock exists if there is already an interrupt raised by the
hardware when the async interrupts are enabled: the GPIO won't raise an interrupt as it
will not see the High-Low transition but nothing will clear the existing interrupt.
A "dummy" read of INTFB through a call to [`PiFaceDigital::get_interrupt_capture()`]
will ensure any existing interrupts are cleared.

# Example usage

```rust no_run
use anyhow::Result;
use log::{error, info};
use rppal_pfd::{
    ChipSelect, HardwareAddress, InterruptMode, Level, PiFaceDigital, SpiBus, SpiMode,
};
use std::{sync::mpsc::channel, time::Duration};

#[derive(Debug)]
enum HardwareInterfaceMessage {
    InterruptReceived,
}

fn main() -> Result<()> {
    env_logger::init();
    info!("Async blink started!");

    println!("Use the push-buttons to control the blink rate via async interrupts:\n");
    println!("  Button 1:  Faster");
    println!("  Button 2:  Slower");
    println!("  Button 3:  Quit\n");

    let mut pfd = PiFaceDigital::new(
        HardwareAddress::new(0).unwrap(),
        SpiBus::Spi0,
        ChipSelect::Cs0,
        100_000,
        SpiMode::Mode0,
    )?;
    pfd.init()?;

    let mut faster_button = pfd.get_pull_up_input_pin(0)?;
    let mut slower_button = pfd.get_pull_up_input_pin(1)?;
    let mut quit_button = pfd.get_pull_up_input_pin(2)?;
    let led = pfd.get_output_pin_low(2)?;

    faster_button.set_interrupt(InterruptMode::BothEdges)?;
    slower_button.set_interrupt(InterruptMode::BothEdges)?;
    quit_button.set_interrupt(InterruptMode::BothEdges)?;

    let mut period = 1.0;
    let mut led_state = Level::High;
    let mut quit = false;

    let (tx, rx) = channel();

    pfd.subscribe_async_interrupts(move |_| {
        tx.send(HardwareInterfaceMessage::InterruptReceived)
            .expect("Failed to send message");
    })?;

    // If there's an interrupt already active, we'll not detect the next one and never
    // service it, so read INTCAP which will clear it if it is already active.
    let _ = pfd.get_interrupt_capture()?;

    while !quit {
        led.write(led_state)?;

        match rx.recv_timeout(Duration::from_secs_f64(period / 2.0)) {
            Ok(msg) => {
                println!("An interrupt happened (msg={msg:?})...");

                let flags = pfd.get_interrupt_flags()?;
                let inputs = pfd.get_interrupt_capture()?;
                if (flags & 0x01) != 0 {
                    println!("Got button 1 (0x{flags:02x})");
                    if (inputs & 0x01) == 0 {
                        period /= 2.0;
                    }
                } else if (flags & 0x02) != 0 {
                    println!("Got button 2 (0x{flags:02x})");
                    if (inputs & 0x02) == 0 {
                        period *= 2.0;
                    }
                } else if (flags & 0x04) != 0 {
                    println!("Got button 3 (0x{flags:02x})");
                    if (inputs & 0x04) == 0 {
                        quit = true;
                    }
                } else {
                    error!("Got unmatched 0x{flags:02x}");
                }
            }
            Err(_) => {
                led_state = !led_state;
            }
        }
    }

    // Quit, leaving the LED off.
    led.set_low()?;
    println!("\nBlinking is done!\n");
    Ok(())
}
```

# Testing

Note that in testing environments or with the `mockspi` feature enabled, this
is replaced with a dummy function that does not spawn a thread and will never invoke
the callback function.
