# PiFace Digital driver

![Crates.io](https://img.shields.io/crates/v/rppal-pfd)
![Crates.io](https://img.shields.io/crates/d/rppal-pfd)
![Crates.io](https://img.shields.io/crates/l/rppal-pfd)
![GitHub Workflow Status](https://img.shields.io/github/workflow/status/solimike/rppal-pifacedigital/todo)

A driver for the PiFace Digital I/O expander for the
[Raspberry Pi](https://www.raspberrypi.org/) which is accessed over an SPI bus.

## Example usage

``` rust no_run
use rppal_pfd::{ChipSelect, HardwareAddress, Level, PiFaceDigital, SpiBus, SpiMode};

// Create an instance of the driver for the device with the hardware address
// (A1, A0) of 0b00 on SPI bus 0 clocked at 100kHz. The address bits are set using
// `JP1` and `JP2` on the PiFace Digital board.
let pfd = PiFaceDigital::new(
    HardwareAddress::new(0).expect("Invalid hardware address"),
    SpiBus::Spi0,
    ChipSelect::Cs0,
    100_000,
    SpiMode::Mode0,
)
.expect("Failed to create PiFace Digital");

// Take ownership of the output pin on bit 4 of the device.
let pin = pfd
    .get_output_pin(4)
    .expect("Failed to get Pin");

// Set the pin to logic-level low.
pin.write(Level::Low).expect("Bad pin write");
```

## Concurrency Warning

Note that the `rppal_mcp23s17` contained in the [`PiFaceDigital`] is
[`!Send`](std::marker::Send) so that the device can only be used within the
context of a single thread. However, there is nothing to stop separate instances on
separate threads accessing the same MCP23S17 device.  However, when it comes to the
PiFace Digital itself, it needs to take ownership of the Raspberry PI's `GPIO-25`
pin which is used as the interrupt input. As it currently stands that has the effect
of enforcing the existence of just one PiFace Digital device on the system because
attempts to create a second device will fail with a "GPIO device busy" error.

Further work is necessary to allow a single process to share the interrupts; sharing
between processes is likely always going to be impossible with this user-space
architecture for the interrupts.

## Acknowledgements

This library has taken a lot of inspiration and guidance from the design of the
[PiFace Digital I/O Python library](https://github.com/piface/pifacedigitalio).

This library has followed some of the API design patterns used in the
[RPPAL crate](https://crates.io/crates/rppal).

Thanks!
