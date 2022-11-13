// Blink an LED, controlling the flash-rate with buttons.
//
// This example illustrates:
//
// - OutputPin to flash the LED.
// - InputPin to detect buttons.
// - Polling for interrupts across multiple InputPins.
// - Use of a timeout for the interrupt polling.
//
// USAGE:
//
// The four buttons control the operation:
//
// 1) Flash faster
// 2) Flash slower
// 3) Quit the program
// 4) [Not used]

use anyhow::Result;
use log::{error, info};
use rppal_pfd::{
    ChipSelect, HardwareAddress, InterruptMode, Level, PiFaceDigital, SpiBus, SpiMode,
};
use std::{cmp::max, ptr, time::Duration};

fn main() -> Result<()> {
    env_logger::init();
    info!("Blink started!");

    println!("Use the push-buttons to control the blink rate:\n");
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

    // Generate interrupts on both edges as this simplifies the logic
    // to avoid perpetual re-interrupts whilst the button is pressed.
    faster_button.set_interrupt(InterruptMode::BothEdges)?;
    slower_button.set_interrupt(InterruptMode::BothEdges)?;
    quit_button.set_interrupt(InterruptMode::BothEdges)?;

    let mut period = 1000;
    let mut led_state = Level::High;
    let mut quit = false;

    while !quit {
        led.write(led_state)?;

        match pfd.poll_interrupts(
            &[&faster_button, &slower_button, &quit_button],
            false,
            Some(Duration::from_millis(period / 2)),
        ) {
            // At least one (most probably exactly one) button interrupted so action it.
            Ok(Some(interrupts)) => {
                for (pin, level) in interrupts {
                    match pin {
                        p if ptr::eq(p, &faster_button) && level == Level::Low => {
                            period = max(period / 2, 125);
                            println!("Going faster: {} Hz", 1000.0 / period as f32);
                        }

                        p if ptr::eq(p, &slower_button) && level == Level::Low => {
                            period *= 2;
                            println!("Going slower: {} Hz", 1000.0 / period as f32);
                        }

                        p if ptr::eq(p, &quit_button) && level == Level::Low => {
                            quit = true;
                        }

                        p => {
                            info!(
                                "Ignoring button: {} going {}",
                                p.get_pin_number() + 1,
                                level
                            );
                        }
                    }
                }
            }

            // Poll timed out, so invert the state of the LED
            Ok(None) => {
                led_state = !led_state;
            }

            // Oops!
            Err(e) => {
                error!("Poll failed unexpectedly with {e}");
                return Err(e.into());
            }
        }
    }

    // Quit, leaving the LED off.
    led.set_low()?;
    println!("\nBlinking is done!\n");
    Ok(())
}
