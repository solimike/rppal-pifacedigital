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
