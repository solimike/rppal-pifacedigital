#![deny(missing_docs)]
#![doc = include_str!("../README.md")]
//!
//! ## Extended example
//!
//! This example is available in `${CARGO_MANIFEST_DIR}/examples/blink-fast-slow.rs`.
//!
//! ``` rust no_run
#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/blink-fast-slow.rs"))]
//! ```

use std::{
    cell::RefCell,
    fmt::{self, Write},
    rc::Rc,
    result,
    time::Duration,
};

#[cfg(not(any(test, feature = "mockspi")))]
use log::warn;
use log::{debug, error, info, log_enabled, Level::Debug};
#[cfg(not(any(test, feature = "mockspi")))]
use rppal::gpio::{self, Gpio, Trigger};
#[cfg(not(feature = "mockspi"))]
use rppal_mcp23s17::{Mcp23s17, RegisterAddress, IOCON};
#[cfg(feature = "mockspi")]
pub use rppal_mcp23s17::{Mcp23s17, RegisterAddress, IOCON};

use thiserror::Error;

/// Re-export of `rppal_mcp23s17` crate APIs which we use on this crate's APIs.
pub use rppal_mcp23s17::{ChipSelect, InterruptMode, Level, OutputPin, SpiBus, SpiMode};

//--------------------------------------------------------------------------------------
/// The hardware address of the device - two bits.
///
/// The MCP23S17 supports three hardware address bits but the PiFace Digital only exposes
/// `A0` and `A1` on `JP1` and `JP2` respectively.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct HardwareAddress(u8);

impl HardwareAddress {
    /// Hardware address space is two bits wide so 0-3 are valid.
    pub const MAX_HARDWARE_ADDRESS: u8 = 3;

    /// Create a HardwareAddress bounds-checking that it is valid.
    pub fn new(address: u8) -> Result<Self> {
        if address <= Self::MAX_HARDWARE_ADDRESS {
            Ok(Self(address))
        } else {
            Err(PiFaceDigitalError::HardwareAddressBoundsError(address))
        }
    }
}

impl TryFrom<u8> for HardwareAddress {
    type Error = PiFaceDigitalError;

    fn try_from(value: u8) -> Result<Self> {
        HardwareAddress::new(value)
    }
}

impl From<HardwareAddress> for rppal_mcp23s17::HardwareAddress {
    fn from(addr: HardwareAddress) -> Self {
        // A PiFace Digital address is smaller than an MCP23S17 address.
        rppal_mcp23s17::HardwareAddress::new(addr.0).unwrap()
    }
}

impl From<HardwareAddress> for u8 {
    fn from(addr: HardwareAddress) -> Self {
        addr.0
    }
}

impl TryFrom<rppal_mcp23s17::HardwareAddress> for HardwareAddress {
    type Error = PiFaceDigitalError;

    fn try_from(value: rppal_mcp23s17::HardwareAddress) -> Result<Self> {
        Self::try_from(u8::from(value))
    }
}

impl fmt::Display for HardwareAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&format!("{}", self.0), f)
    }
}

//--------------------------------------------------------------------------------------

/// Errors that operation of the PiFace Digital can raise.
#[derive(Error, Debug)]
pub enum PiFaceDigitalError {
    /// Errors from the `rppal_mcp23s17::Mcp23s17`.
    #[error("MCP23S17 error")]
    Mcp23s17Error {
        /// Underlying error source.
        #[from]
        source: rppal_mcp23s17::Mcp23s17Error,
    },

    /// Attempt to access a PiFace Digital beyond the hardware address range
    /// (0 - [`HardwareAddress::MAX_HARDWARE_ADDRESS`]).
    #[error("Hardware address out of range")]
    HardwareAddressBoundsError(u8),

    /// Failed to detect the presence of any physical PiFace Digital device connected to
    /// the SPI bus.
    #[error("No hardware connected to {spi_bus} at hardware address={hardware_address})")]
    NoHardwareDetected {
        /// SPI bus we tried to access the device over.
        spi_bus: SpiBus,
        /// Hardware address of the device we tried to access.
        hardware_address: HardwareAddress,
    },

    /// Errors accessing the GPIO for the interrupt input.
    #[error("GPIO error")]
    GpioError {
        /// Underlying error source.
        #[from]
        source: rppal::gpio::Error,
    },
}

/// Convenient alias for [`Result<_>`] types can have [`PiFaceDigitalError`]s.
pub type Result<T> = result::Result<T, PiFaceDigitalError>;

/// An input pin.
///
/// The [`InputPin`] exposes the capabilities of the underlying `rppal_mcp23s17::InputPin`
/// with the addition of interrupt handling.
///
/// # Example usage
///
/// ```no_run
/// # use rppal_pfd::{ChipSelect, HardwareAddress, Level, PiFaceDigital, SpiBus, SpiMode};
/// #
/// # // Create an instance of the driver for the device with the hardware address
/// # // (A1, A0) of 0b00 on SPI bus 0 clocked at 100kHz. The address bits are set using
/// # // `JP1` and `JP2` on the PiFace Digital board.
/// # let pfd = PiFaceDigital::new(
/// #     HardwareAddress::new(0).expect("Invalid hardware address"),
/// #     SpiBus::Spi0,
/// #     ChipSelect::Cs0,
/// #     100_000,
/// #     SpiMode::Mode0,
/// # )
/// # .expect("Failed to create PiFace Digital");
/// #
/// // Given an instance of a PiFaceDigital, take ownership of the output pin on bit 4
/// // of the device.
/// let pin = pfd
///     .get_output_pin(4)
///     .expect("Failed to get Pin");
///
/// // Set the pin to logic-level low.
/// pin.write(Level::Low).expect("Bad pin write");
/// ```
#[derive(Debug)]
pub struct InputPin {
    pin: rppal_mcp23s17::InputPin,
    interrupts_enabled: bool,
    #[allow(dead_code)] // in test config no functionality accesses pfd_state.
    pfd_state: Rc<RefCell<PiFaceDigitalState>>,
}

/// Internal state of the PiFace Digital card.
#[derive(Debug)]
pub struct PiFaceDigitalState {
    mcp23s17: Mcp23s17,
    #[cfg(not(any(test, feature = "mockspi")))]
    _gpio: Gpio,
    #[cfg(not(any(test, feature = "mockspi")))]
    interrupt_pin: gpio::InputPin,
}

/// Represents an instance of the PiFace Digital I/O expander for the Raspberry Pi.
///
/// This is the key entrypoint into the driver. This driver is a thin wrapper around
/// the `rppal_mcp23s17` driver that is responsible for ensuring that the I/O
/// expander chip is configured in a manner compatible with the capabilities of the
/// PiFace Digital hardware.
///
/// The PiFace Digital has two GPIO ports:
///
/// - `GPIOA` configured as outputs to:
///   - two relays (bits 0 and 1)
///   - surface-mount LEDs on each output
///   - an 8-way terminal block
///
/// - `GPIOB` configured as inputs connected to
///   - four on-board push switches on bits 0-3
///   - an 8-way terminal block
///
/// The user should instantiate a [`PiFaceDigital`] and then use
/// [`PiFaceDigital::get_input_pin()`] and [`PiFaceDigital::get_output_pin()`] to acquire
/// an [`InputPin`] or [`OutputPin`].
///
/// ```no_run
/// use rppal_pfd::{ChipSelect, HardwareAddress, PiFaceDigital, SpiBus, SpiMode};
///
/// // Create an instance of the driver for the device with the hardware address
/// // (A1, A0) of 0b00 on SPI bus 0 clocked at 100kHz. The address bits are set using
/// // JP1 and JP2 on the PiFace Digital board.
/// let pfd = PiFaceDigital::new(
///     HardwareAddress::new(0).expect("Invalid hardware address"),
///     SpiBus::Spi0,
///     ChipSelect::Cs0,
///     100_000,
///     SpiMode::Mode0,
/// )
/// .expect("Failed to create PiFace Digital");
///
/// // Take ownership of the output pin on bit 4 of the device.
/// let pin = pfd
///     .get_output_pin(4)
///     .expect("Failed to get OutputPin");
/// ```
#[derive(Debug)]
pub struct PiFaceDigital {
    pfd_state: Rc<RefCell<PiFaceDigitalState>>,
}

impl PiFaceDigital {
    /// Create a PiFace Digital instance.
    pub fn new(
        address: HardwareAddress,
        spi_bus: SpiBus,
        chip_select: ChipSelect,
        spi_clock: u32,
        spi_mode: SpiMode,
    ) -> Result<Self> {
        let mcp23s17 = Mcp23s17::new(address.into(), spi_bus, chip_select, spi_clock, spi_mode)?;
        #[cfg(any(test, feature = "mockspi"))]
        let pfd_state = PiFaceDigitalState { mcp23s17 };
        #[cfg(not(any(test, feature = "mockspi")))]
        let pfd_state = {
            let gpio = Gpio::new()?;
            let interrupt_pin = gpio.get(25)?.into_input();
            PiFaceDigitalState {
                mcp23s17,
                _gpio: gpio,
                interrupt_pin,
            }
        };
        Ok(PiFaceDigital {
            pfd_state: Rc::new(RefCell::new(pfd_state)),
        })
    }

    /// Initialise the PiFace Digital I/O board.
    ///
    /// Ensures that the registers in the MCP23S17 are configured appropriately for the
    /// hardware setup. In normal use, this function should always be called immediately
    /// after construction of the [`PiFaceDigital`] instance. Only makes sense _not_ to
    /// initialise the device in situations where there are multiple instances and you
    /// can guarantee another instance has initialised the device and you don't want
    /// your instance to overwrite what may now be non-default config (_e.g._ because
    /// pins have been constructed.)
    ///
    /// ## Default settings
    ///
    /// | Register  | Purpose              | GPIO-A side<br>(Output) | GPIO-B side<br>(Input) |
    /// |-----------|----------------------|:-----------:|:-----------:|
    /// |  IODIR    | I/O direction        | 0x00        | 0xFF        |
    /// |  IPOL     | Input polarity       | 0x00        | 0x00        |
    /// |  GPINTEN  | GPIO interrupt enable| 0x00        | 0x00        |
    /// |  DEFVAL   | Default value        | 0x00        | 0x00        |
    /// |  INTCON   | Interrupt control    | 0x00        | 0x00        |
    /// |  IOCON    | I/O control          | 0x28        | (Note 1)    |
    /// |  GPPU     | GPIO pull-up         | 0x00        | 0xFF<br>(Note 3) |
    /// |  INTF     | Interrupt Flag       |             |             |
    /// |  INTCAP   | Interrupt capture    |             |             |
    /// |  GPIO     | Input/Output         | 0x00        |             |
    /// |  OLAT     | Output latch         | (Note 2)    |(Note 2)     |
    ///
    /// **Notes:**
    ///
    /// 1) There is only one IOCON register (though it appears at two addresses).
    ///    The value written into IOCON represents:
    ///
    ///     - `BANK` off
    ///     - `MIRROR` off
    ///     - `SEQOP` off
    ///     - `DISSLW` enabled (this is an I<sup>2</sup>C function so effectively "don't care")
    ///     - `HAEN` on
    ///     - `ODR` off
    ///     - `INTPOL` active low
    ///
    ///    The duplicate IOCON register is not written by this function.
    ///
    /// 2) OLATA is not explicitly written by this function but the value written to
    ///    GPIOA is also written through to OLATA by the device itself.
    ///
    /// 3) May mean that active digital inputs see an inappropriate pull-up load on
    ///    initialisation, but avoids having floating inputs picking up noise (and,
    ///    anyway, this is what the four push-switches on `GPIOB-0` to `GPIOB-3` are
    ///    expecting!)
    ///
    /// Once the MCP23S17 is in the desired state, the interrupt line on the Raspberry
    /// Pi's GPIO gets enabled.
    pub fn init(&mut self) -> Result<()> {
        info!("Initialise PiFaceDigital registers to default values");

        // First ensure IOCON is correct so that register addressing is set appropriately.
        // It can't be done in the table below because the bits() function isn't const.
        let iocon = (IOCON::BANK_OFF
            | IOCON::MIRROR_OFF
            | IOCON::SEQOP_OFF
            | IOCON::DISSLW_SLEW_RATE_CONTROLLED
            | IOCON::HAEN_ON
            | IOCON::ODR_OFF
            | IOCON::INTPOL_LOW)
            .bits();
        self.pfd_state
            .borrow()
            .mcp23s17
            .write(RegisterAddress::IOCON, iocon)?;

        // There are no acknowledgements in the SPI protocol so read-back the value to
        // assess whether there's actually anything connected.
        if self
            .pfd_state
            .borrow()
            .mcp23s17
            .read(RegisterAddress::IOCON)?
            != iocon
        {
            return Err(PiFaceDigitalError::NoHardwareDetected {
                spi_bus: self.pfd_state.borrow().mcp23s17.get_spi_bus(),
                hardware_address: self
                    .pfd_state
                    .borrow()
                    .mcp23s17
                    .get_hardware_address()
                    .try_into()
                    .expect("MCP23S17 hardware address limited to PiFace Digital range"),
            });
        }

        // Log debug info about the current register state.
        debug!("Uninitialised MCP23S17 state");
        self.debug_current_state("Uninitialised MCP23S17 state:")?;

        const RESET_REGISTER_STATES: [(RegisterAddress, Option<u8>); RegisterAddress::LENGTH] = [
            (RegisterAddress::IODIRA, Some(0x00)),
            (RegisterAddress::IODIRB, Some(0xFF)),
            (RegisterAddress::IPOLA, Some(0x00)),
            (RegisterAddress::IPOLB, Some(0x00)),
            (RegisterAddress::GPINTENA, Some(0x00)),
            (RegisterAddress::GPINTENB, Some(0x00)),
            (RegisterAddress::DEFVALA, Some(0x00)),
            (RegisterAddress::DEFVALB, Some(0x00)),
            (RegisterAddress::INTCONA, Some(0x00)),
            (RegisterAddress::INTCONB, Some(0x00)),
            (RegisterAddress::IOCON, None),
            (RegisterAddress::IOCON2, None),
            (RegisterAddress::GPPUA, Some(0x00)),
            (RegisterAddress::GPPUB, Some(0xFF)),
            (RegisterAddress::INTFA, None),
            (RegisterAddress::INTFB, None),
            (RegisterAddress::INTCAPA, None),
            (RegisterAddress::INTCAPB, None),
            (RegisterAddress::GPIOA, Some(0x00)),
            (RegisterAddress::GPIOB, None),
            (RegisterAddress::OLATA, None),
            (RegisterAddress::OLATB, None),
        ];

        for (register_address, default_value) in RESET_REGISTER_STATES {
            if let Some(data) = default_value {
                self.pfd_state
                    .borrow()
                    .mcp23s17
                    .write(register_address, data)?;
                debug!("New {register_address:?} register state: 0x{data:02x}");
            }
        }

        // Log debug info about the updated register state.
        debug!("Initialised MCP23S17 state");
        self.debug_current_state("Initialised MCP23S17 state:")?;

        // Enable the GPIO interrupts. The MCP23S17 should be in a state where all
        // interrupts are disabled so there shouldn't be an immediate trigger.
        #[cfg(not(any(test, feature = "mockspi")))]
        self.pfd_state
            .borrow_mut()
            .interrupt_pin
            .set_interrupt(Trigger::FallingEdge)?;

        Ok(())
    }

    /// Returns an [`InputPin`] for the specified pin number configured as a
    /// high-impedance input.
    ///
    /// If the pin is already in use, or the pin number `pin` is greater than 7 then
    /// `get_input_pin()` returns `Err(`[`PiFaceDigitalError::Mcp23s17Error`]`)`
    /// with the source error type of `rppal_mcp23s17::Mcp23s17Error::PinNotAvailable`.
    ///
    /// After the [`InputPin`] goes out of scope, it can be retrieved again through
    /// another `get_input_pin()` call.
    ///
    /// When constructed, the pin has interrupts disabled.
    pub fn get_input_pin(&self, pin: u8) -> Result<InputPin> {
        // Get the unconfigured Pin (assuming it's available) and then convert it to
        // an InputPin (i.e. high-impedance input).
        Ok(InputPin {
            pin: self
                .pfd_state
                .borrow()
                .mcp23s17
                .get(rppal_mcp23s17::Port::GpioB, pin)?
                .into_input_pin()?,
            interrupts_enabled: false,
            pfd_state: self.pfd_state.clone(),
        })
    }

    /// Returns an [`InputPin`] for the specified pin number configured with a pull-up
    /// resistor.
    ///
    /// If the pin is already in use, or the pin number `pin` is greater than 7 then
    /// `get_input_pin()` returns `Err(`[`PiFaceDigitalError::Mcp23s17Error`]`)`
    /// with the source error type of `rppal_mcp23s17::Mcp23s17Error::PinNotAvailable`.
    ///
    /// After the [`InputPin`] goes out of scope, it can be retrieved again through
    /// another `get_input_pin()` call.
    ///
    /// When constructed, the pin has interrupts disabled.
    pub fn get_pull_up_input_pin(&self, pin: u8) -> Result<InputPin> {
        // Get the unconfigured Pin (assuming it's available) and then convert it to
        // an InputPin (i.e. high-impedance input).
        Ok(InputPin {
            pin: self
                .pfd_state
                .borrow()
                .mcp23s17
                .get(rppal_mcp23s17::Port::GpioB, pin)?
                .into_pullup_input_pin()?,
            interrupts_enabled: false,
            pfd_state: self.pfd_state.clone(),
        })
    }

    /// Returns an [`OutputPin`] for the specified pin number.
    ///
    /// If the pin is already in use, or the pin number `pin` is greater than 7 then
    /// `get_input_pin()` returns `Err(`[`PiFaceDigitalError::Mcp23s17Error`]`)`
    /// with the source error type of `rppal_mcp23s17::Mcp23s17Error::PinNotAvailable`.
    ///
    /// After the [`OutputPin`] goes out of scope, it can be retrieved again through
    /// another `get_output_pin()` call.
    pub fn get_output_pin(&self, pin: u8) -> Result<OutputPin> {
        // Get the unconfigured Pin (assuming it's available) and then convert it to
        // an InputPin (i.e. high-impedance input).
        Ok(self
            .pfd_state
            .borrow()
            .mcp23s17
            .get(rppal_mcp23s17::Port::GpioA, pin)?
            .into_output_pin()?)
    }

    /// Returns an [`OutputPin`] for the specified pin number already set high.
    ///
    /// If the pin is already in use, or the pin number `pin` is greater than 7 then
    /// `get_input_pin()` returns `Err(`[`PiFaceDigitalError::Mcp23s17Error`]`)`
    /// with the source error type of `rppal_mcp23s17::Mcp23s17Error::PinNotAvailable`.
    ///
    /// After the [`OutputPin`] goes out of scope, it can be retrieved again through
    /// another `get_output_pin()` call.
    pub fn get_output_pin_high(&self, pin: u8) -> Result<OutputPin> {
        // Get the unconfigured Pin (assuming it's available) and then convert it to
        // an InputPin (i.e. high-impedance input).
        Ok(self
            .pfd_state
            .borrow()
            .mcp23s17
            .get(rppal_mcp23s17::Port::GpioA, pin)?
            .into_output_pin_high()?)
    }

    /// Returns an [`OutputPin`] for the specified pin number already set low.
    ///
    /// If the pin is already in use, or the pin number `pin` is greater than 7 then
    /// `get_input_pin()` returns `Err(`[`PiFaceDigitalError::Mcp23s17Error`]`)`
    /// with the source error type of `rppal_mcp23s17::Mcp23s17Error::PinNotAvailable`.
    ///
    /// After the [`OutputPin`] goes out of scope, it can be retrieved again through
    /// another `get_output_pin()` call.
    pub fn get_output_pin_low(&self, pin: u8) -> Result<OutputPin> {
        // Get the unconfigured Pin (assuming it's available) and then convert it to
        // an InputPin (i.e. high-impedance input).
        Ok(self
            .pfd_state
            .borrow()
            .mcp23s17
            .get(rppal_mcp23s17::Port::GpioA, pin)?
            .into_output_pin_high()?)
    }

    /// Waits for interrupts across a set of InputPins.
    ///
    /// Blocks until an interrupt is raised and then returns a list of references to the
    /// pin or pins that were the source. Note that it is possible that none of the
    /// pins were actually the source of the error (though this is suspicious and will
    /// cause a warning log) in which case the returned vector will be empty.
    ///
    /// Each interrupt is represented by a tuple of a reference to the pin and the level
    /// on the pin when the interrupt happened.
    ///
    /// If the timeout expires the function will return [`None`].
    ///
    /// # Example usage
    ///
    /// ```no_run
    /// use rppal_pfd::{ChipSelect, HardwareAddress, InterruptMode, PiFaceDigital, SpiBus, SpiMode};
    /// use std::time::Duration;
    ///
    /// // Create an instance of the driver for the device with the hardware address
    /// // (A1, A0) of 0b00 on SPI bus 0 clocked at 100kHz. The address bits are set using
    /// // JP1 and JP2 on the PiFace Digital board.
    /// let mut pfd = PiFaceDigital::new(
    ///     HardwareAddress::new(0).expect("Invalid hardware address"),
    ///     SpiBus::Spi0,
    ///     ChipSelect::Cs0,
    ///     100_000,
    ///     SpiMode::Mode0,
    /// )
    /// .expect("Failed to create PiFace Digital");
    ///
    /// // Creating interrupt pin on the fourth switch on the PiFace Digital card.
    /// let mut interrupt_pin1 = pfd.get_pull_up_input_pin(3).expect("Bad pin");
    /// interrupt_pin1.set_interrupt(InterruptMode::BothEdges).expect("Bad interrupt");
    ///
    /// // Creating interrupt pin on the third switch on the PiFace Digital card.
    /// let mut interrupt_pin2 = pfd.get_pull_up_input_pin(2).expect("Bad pin");
    /// interrupt_pin2.set_interrupt(InterruptMode::BothEdges).expect("Bad interrupt");
    ///
    /// loop {
    ///     // Wait one minute for a button press...
    ///     match pfd.poll_interrupts(
    ///         &[&interrupt_pin1, &interrupt_pin2],
    ///         false,
    ///         Some(Duration::from_secs(60)),
    ///     ) {
    ///         Ok(Some(interrupts)) => {
    ///             // Button(s) were pressed!
    ///             for (i, (pin, level)) in interrupts.iter().enumerate() {
    ///                 let pin_no = pin.get_pin_number();
    ///                 println!("Interrupt[{i}]: pin({pin_no}) is {level}");
    ///             }
    ///         }
    ///
    ///         Ok(None) => {
    ///             println!("Poll timed out");
    ///             break;
    ///         }
    ///
    ///         Err(e) => {
    ///             eprintln!("Poll failed with {e}");
    ///             break;
    ///         }
    ///     }
    /// }
    /// ```
    #[cfg(not(any(test, feature = "mockspi")))]
    pub fn poll_interrupts<'a>(
        &self,
        pins: &[&'a InputPin],
        reset: bool,
        timeout: Option<Duration>,
    ) -> Result<Option<Vec<(&'a InputPin, Level)>>> {
        // Including a pin that can't raise interrupts is considered a coding error.
        for pin in pins {
            assert!(
                pin.interrupts_enabled(),
                "InputPin({}) included in poll() does not have interrupts enabled!",
                pin.get_pin_number()
            );
        }

        let mut pfd_state = self.pfd_state.borrow_mut();

        match pfd_state.interrupt_pin.poll_interrupt(reset, timeout)? {
            Some(_level) => {
                // There was an interrupt so work out what pin/pins registered it and
                // get the input capture so we can report the levels on the pins.
                let interrupt_flags = pfd_state.mcp23s17.read(RegisterAddress::INTFB)?;
                let input_port = pfd_state.mcp23s17.read(RegisterAddress::INTCAPB)?;
                let mut interrupting_pins = Vec::new();

                for pin in pins {
                    let pin_no = pin.get_pin_number();
                    if (interrupt_flags & (0x01 << pin_no)) != 0 {
                        let level: Level = (input_port & (0x01 << pin_no)).into();
                        debug!("Active interrupt on pin {pin_no} level {level}");
                        interrupting_pins.push((*pin, level));
                    }
                }

                // Finding no active interrupts may be intentional but most likely
                // indicates a misconfiguration, so log a warning.
                if interrupting_pins.is_empty() {
                    warn!(
                        "No interrupts on any of pins {pins:?} - will poll again but interrupt will have been lost!"
                    );
                }
                Ok(Some(interrupting_pins))
            }

            // Poll timed out.
            None => Ok(None),
        }
    }

    /// Waits for interrupts across a set of InputPins.
    ///
    /// Blocks until an interrupt is raised and then returns a list of references to the
    /// pin or pins that were the source. Note that it is possible that none of the
    /// pins were actually the source of the error (though this is suspicious and will
    /// cause a warning log) in which case the returned vector will be empty.
    ///
    /// Each interrupt is represented by a tuple of a reference to the pin and the level
    /// on the pin when the interrupt happened.
    ///
    /// If the timeout expires the function will return [`None`].
    ///
    /// # Example usage
    ///
    /// ```no_run
    /// use rppal_pfd::{ChipSelect, HardwareAddress, InterruptMode, PiFaceDigital, SpiBus, SpiMode};
    /// use std::time::Duration;
    ///
    /// // Create an instance of the driver for the device with the hardware address
    /// // (A1, A0) of 0b00 on SPI bus 0 clocked at 100kHz. The address bits are set using
    /// // JP1 and JP2 on the PiFace Digital board.
    /// let mut pfd = PiFaceDigital::new(
    ///     HardwareAddress::new(0).expect("Invalid hardware address"),
    ///     SpiBus::Spi0,
    ///     ChipSelect::Cs0,
    ///     100_000,
    ///     SpiMode::Mode0,
    /// )
    /// .expect("Failed to create PiFace Digital");
    ///
    /// // Creating interrupt pin on the fourth switch on the PiFace Digital card.
    /// let mut interrupt_pin1 = pfd.get_pull_up_input_pin(3).expect("Bad pin");
    /// interrupt_pin1.set_interrupt(InterruptMode::BothEdges).expect("Bad interrupt");
    ///
    /// // Creating interrupt pin on the third switch on the PiFace Digital card.
    /// let mut interrupt_pin2 = pfd.get_pull_up_input_pin(2).expect("Bad pin");
    /// interrupt_pin2.set_interrupt(InterruptMode::BothEdges).expect("Bad interrupt");
    ///
    /// loop {
    ///     // Wait one minute for a button press...
    ///     match pfd.poll_interrupts(
    ///         &[&interrupt_pin1, &interrupt_pin2],
    ///         false,
    ///         Some(Duration::from_secs(60)),
    ///     ) {
    ///         Ok(Some(interrupts)) => {
    ///             // Button(s) were pressed!
    ///             for (i, (pin, level)) in interrupts.iter().enumerate() {
    ///                 let pin_no = pin.get_pin_number();
    ///                 println!("Interrupt[{i}]: pin({pin_no}) is {level}");
    ///             }
    ///         }
    ///
    ///         Ok(None) => {
    ///             println!("Poll timed out");
    ///             break;
    ///         }
    ///
    ///         Err(e) => {
    ///             eprintln!("Poll failed with {e}");
    ///             break;
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// # Testing
    ///
    /// Note that in testing environments or with the `mockspi` feature enabled, this
    /// is replaced with a dummy interrupt poll function that always returns as if a
    /// timeout occurred (even if specified timeout was "forever", which is frankly
    /// wrong!)
    #[cfg(any(test, feature = "mockspi"))]
    pub fn poll_interrupts<'a>(
        &self,
        pins: &[&'a InputPin],
        _reset: bool,
        _timeout: Option<Duration>,
    ) -> Result<Option<Vec<(&'a InputPin, Level)>>> {
        for pin in pins {
            assert!(
                pin.interrupts_enabled(),
                "InputPin({}) included in poll() does not have interrupts enabled!",
                pin.get_pin_number()
            );
        }
        let mut _pfd_state = self.pfd_state.borrow_mut();

        Ok(None)
    }

    /// Generate a debug log containing the state of the MCP23S17.
    ///
    /// If logging at `Debug` level, log the values currently in the MCP23S17's
    /// registers, otherwise does nothing.
    pub fn debug_current_state(&self, context: &str) -> Result<()> {
        if log_enabled!(Debug) {
            let mut state = String::new();
            for register in 0..RegisterAddress::LENGTH {
                let register_address = RegisterAddress::try_from(register).unwrap();
                let data = self.pfd_state.borrow().mcp23s17.read(register_address)?;
                writeln!(state, "{:10} : 0x{:02x}", register_address, data).unwrap();
            }
            debug!("{context}\n{state}");
        }
        Ok(())
    }

    /// In testing environments provide an API to get access to the mock SPI that
    /// allows unit tests to be run without a real Raspberry Pi.
    ///
    /// Returns a tuple containing:
    ///
    /// - The current data in the mock SPI register `register`.
    /// - The number of read accesses made to the register.
    /// - The number of write accesses made to the register.
    ///
    /// ```
    /// use rppal_pfd::{ChipSelect, HardwareAddress, Level, PiFaceDigital, RegisterAddress, SpiBus, SpiMode};
    ///
    /// let mut pfd = PiFaceDigital::new(
    ///     HardwareAddress::new(0).unwrap(),
    ///     SpiBus::Spi0,
    ///     ChipSelect::Cs0,
    ///     100_000,
    ///     SpiMode::Mode0,
    /// ).expect("Failed to construct!");
    /// pfd.init().expect("Failed to initialise!");
    ///
    /// // The IOCON register gets set once and then read back by the initialise to
    /// // test that there's actually some hardware connected. The 0x28 represents the
    /// // default configuration.
    /// assert_eq!(pfd.get_mock_data(RegisterAddress::IOCON),
    ///     (0x28, 1, 1));
    /// ```

    #[cfg(any(test, feature = "mockspi"))]
    pub fn get_mock_data(&self, register: RegisterAddress) -> (u8, usize, usize) {
        self.pfd_state.borrow().mcp23s17.get_mock_data(register)
    }

    /// In testing environments provide an API to get access to the mock SPI that
    /// allows unit tests to be run without a real Raspberry Pi.
    ///
    /// ```
    /// use rppal_pfd::{ChipSelect, HardwareAddress, Level, PiFaceDigital, RegisterAddress, SpiBus, SpiMode};
    ///
    /// let mut pfd = PiFaceDigital::new(
    ///     HardwareAddress::new(0).unwrap(),
    ///     SpiBus::Spi0,
    ///     ChipSelect::Cs0,
    ///     100_000,
    ///     SpiMode::Mode0,
    /// ).expect("Failed to construct!");
    ///
    /// pfd.set_mock_data(RegisterAddress::IOCON, 0x55);
    /// assert_eq!(pfd.get_mock_data(RegisterAddress::IOCON),
    ///     (0x55, 0, 0));
    ///
    /// pfd.init().expect("Failed to initialise!");
    ///
    /// // The IOCON register gets set once and then read back by the initialise to
    /// // test that there's actually some hardware connected. The 0x28 represents the
    /// // default configuration.
    /// assert_eq!(pfd.get_mock_data(RegisterAddress::IOCON),
    ///     (0x28, 1, 1));
    /// ```
    #[cfg(any(test, feature = "mockspi"))]
    pub fn set_mock_data(&self, register: RegisterAddress, data: u8) {
        self.pfd_state
            .borrow()
            .mcp23s17
            .set_mock_data(register, data)
    }
}

impl InputPin {
    /// Reads the pin's logic level.
    #[inline]
    pub fn read(&self) -> Result<Level> {
        Ok(self.pin.read()?)
    }

    /// Reads the pin's logic level, and returns [`true`] if it is set to
    /// [`Level::Low`].
    #[inline]
    pub fn is_low(&self) -> Result<bool> {
        Ok(self.pin.read()? == Level::Low)
    }

    /// Reads the pin's logic level, and returns [`true`] if it is set to
    /// [`Level::High`].
    #[inline]
    pub fn is_high(&self) -> Result<bool> {
        Ok(self.pin.read()? == Level::High)
    }

    /// Enable synchronous interrupts.
    ///
    /// Synchronous interrupts can be polled once enabled by either:
    ///
    /// - Calling [`InputPin::poll_interrupt()`] in the case where just one `InputPin`
    ///   is configured to raise interrupts.
    /// - Calling [`PiFaceDigital::poll_interrupts()`] in the case where more than one
    ///   InputPin is configured to raise interrupts.
    ///
    /// Interrupts can be disabled by a call to [`InputPin::clear_interrupt()`] and
    /// will also be automatically disabled when the `InputPin` is dropped.
    pub fn set_interrupt(&mut self, mode: InterruptMode) -> Result<()> {
        self.interrupts_enabled = true;
        self.pin.set_interrupt_mode(mode).map_err(|e| e.into())
    }

    /// Disable synchronous interrupts on the pin.
    ///
    /// Note that:
    ///
    /// - Multiple calls to `clear_interrupt()` are permitted (though don't do anything
    ///   useful).
    /// - If not explicitly disabled, the interrupts will be disabled when the pin
    ///   is dropped.
    pub fn clear_interrupt(&mut self) -> Result<()> {
        self.interrupts_enabled = false;
        self.pin
            .set_interrupt_mode(InterruptMode::None)
            .map_err(|e| e.into())
    }

    /// Wait for an interrupt (or timeout) on this pin.
    ///
    /// Must only be called if interrupts have been enabled - calling with interrupts
    /// disabled is considered a coding error and will panic.
    ///
    /// If `reset` is `true` it will cause the GPIO interrupts to be flushed before
    /// starting the poll.
    ///
    /// If no interrupts have happened after `timeout`, the function will exit returning
    /// `Ok(None))`.
    ///
    /// Note that interrupts will have been re-enabled by the time that the poll returns
    /// so there may be repeated interrupts.
    ///
    /// ## Example usage
    ///
    /// ```no_run
    /// use rppal_pfd::{ChipSelect, HardwareAddress, InterruptMode, PiFaceDigital, SpiBus, SpiMode};
    /// use std::time::Duration;
    ///
    ///  let mut pfd = PiFaceDigital::new(
    ///     HardwareAddress::new(0).unwrap(),
    ///     SpiBus::Spi0,
    ///     ChipSelect::Cs0,
    ///     100_000,
    ///     SpiMode::Mode0,
    ///     )
    ///     .expect("Failed to create PFD");
    /// pfd.init().expect("Failed to initialise PFD");
    ///
    /// let mut pin = pfd.get_input_pin(0).expect("Failed to get pin");
    /// pin.set_interrupt(InterruptMode::BothEdges)
    ///     .expect("Failed to enable interrupts");
    ///
    /// match pin.poll_interrupt(false, Some(Duration::from_secs(60))) {
    ///     Ok(Some(level)) => {
    ///         println!("Button pressed!");
    ///         let pin_no = pin.get_pin_number();
    ///         println!("Interrupt: pin({pin_no}) is {level}");
    ///     }
    ///
    ///     Ok(None) => {
    ///         println!("Poll timed out");
    ///     }
    ///
    ///     Err(e) => {
    ///         eprintln!("Poll failed with {e}");
    ///     }     
    /// }
    ///
    #[cfg(not(any(test, feature = "mockspi")))]
    pub fn poll_interrupt(
        &mut self,
        reset: bool,
        timeout: Option<Duration>,
    ) -> Result<Option<Level>> {
        use std::time::Instant;

        assert!(
            self.interrupts_enabled,
            "InputPin({}): No interrupts enabled before trying to poll()",
            self.get_pin_number()
        );

        let wait_until = timeout.map(|delay| Instant::now() + delay);

        // The interrupt line may be asserted by any pin on this device or, potentially,
        // other PiFace Digital devices on the same SPI bus.  Check the relevant INTFB
        // bit to see if this pin caused the interrupt.
        loop {
            let timeout = wait_until.map(|end_time| end_time - Instant::now());
            let mut pfd_state = self.pfd_state.borrow_mut();
            match pfd_state.interrupt_pin.poll_interrupt(reset, timeout)? {
                Some(_level) => {
                    if pfd_state
                        .mcp23s17
                        .get_bit(RegisterAddress::INTFB, self.pin.get_pin_number())?
                        .into()
                    {
                        // We did raise the interrupt condition.
                        info!("Received interrupt on pin {}", self.pin.get_pin_number());
                        return Ok(Some(self.read()?));
                    } else {
                        // Wasn't this pin. We have to read the port to clear the
                        // interrupt but this probably wasn't what was intended so raise
                        // a warning.
                        warn!(
                            "Interrupt was not on pin {} - will poll again but interrupt will have been lost!",
                            self.pin.get_pin_number()
                        );
                        let _ = self.read()?;
                    }
                }
                None => return Ok(None),
            }
        }
    }

    /// Dummy version of interrupt poll routine for use in testing environments.
    ///
    /// Immediately returns as if a timeout occurred.
    #[cfg(any(test, feature = "mockspi"))]
    pub fn poll_interrupt(
        &mut self,
        _reset: bool,
        _timeout: Option<Duration>,
    ) -> Result<Option<Level>> {
        assert!(
            self.interrupts_enabled,
            "InputPin({}): No interrupts enabled before trying to poll()",
            self.get_pin_number()
        );
        Ok(None)
    }

    /// Get the pin number (0-7) that this pin is connected to.
    pub fn get_pin_number(&self) -> u8 {
        self.pin.get_pin_number()
    }

    /// Get the interrupt state.
    pub fn interrupts_enabled(&self) -> bool {
        self.interrupts_enabled
    }
}

impl Drop for InputPin {
    fn drop(&mut self) {
        if self.interrupts_enabled {
            self.clear_interrupt()
                .expect("InputPin failed to clear interrupts on Drop");
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn pfd_input_pin_poll_interrupt() {
        let mut pfd = PiFaceDigital::new(
            HardwareAddress::new(0).unwrap(),
            SpiBus::Spi0,
            ChipSelect::Cs0,
            100_000,
            SpiMode::Mode0,
        )
        .expect("Failed to create PFD");
        pfd.init().expect("Failed to initialise PFD");

        let mut pin = pfd.get_input_pin(0).expect("Failed to get pin");
        pin.set_interrupt(InterruptMode::BothEdges)
            .expect("Failed to enable interrupts");

        assert_eq!(pin.poll_interrupt(false, None).expect("Bad poll"), None);
    }

    #[test]
    #[should_panic]
    fn pfd_input_pin_poll_interrupt_bad_config() {
        let mut pfd = PiFaceDigital::new(
            HardwareAddress::new(0).unwrap(),
            SpiBus::Spi0,
            ChipSelect::Cs0,
            100_000,
            SpiMode::Mode0,
        )
        .expect("Failed to create PFD");
        pfd.init().expect("Failed to initialise PFD");

        let mut pin = pfd.get_input_pin(0).expect("Failed to get pin");

        let _ = pin.poll_interrupt(false, None).expect("Bad poll");
    }

    #[test]
    fn pfd_input_pins_poll_interrupts() {
        let mut pfd = PiFaceDigital::new(
            HardwareAddress::new(0).unwrap(),
            SpiBus::Spi0,
            ChipSelect::Cs0,
            100_000,
            SpiMode::Mode0,
        )
        .expect("Failed to create PFD");
        pfd.init().expect("Failed to initialise PFD");

        let mut pin1 = pfd.get_input_pin(0).expect("Failed to get pin");
        pin1.set_interrupt(InterruptMode::BothEdges)
            .expect("Failed to enable interrupts");
        let mut pin2 = pfd.get_input_pin(1).expect("Failed to get pin");
        pin2.set_interrupt(InterruptMode::BothEdges)
            .expect("Failed to enable interrupts");

        let interrupt_pins = [&pin1, &pin2];
        if let Some(interrupting_pins) = pfd
            .poll_interrupts(&interrupt_pins, false, None)
            .expect("Bad poll")
        {
            panic!("Not expecting any interrupts! Got: {interrupting_pins:?}")
        }
    }

    #[test]
    #[should_panic]
    fn pfd_input_pins_poll_interrupts_bad_config() {
        let mut pfd = PiFaceDigital::new(
            HardwareAddress::new(0).unwrap(),
            SpiBus::Spi0,
            ChipSelect::Cs0,
            100_000,
            SpiMode::Mode0,
        )
        .expect("Failed to create PFD");
        pfd.init().expect("Failed to initialise PFD");

        let mut pin1 = pfd.get_input_pin(0).expect("Failed to get pin");
        pin1.set_interrupt(InterruptMode::BothEdges)
            .expect("Failed to enable interrupts");
        let pin2 = pfd.get_input_pin(1).expect("Failed to get pin");

        let interrupt_pins = [&pin1, &pin2];
        let _ = pfd.poll_interrupts(&interrupt_pins, false, None);
    }

    #[test]
    fn pfd_input_pin_enable_interrupts() {
        let mut pfd = PiFaceDigital::new(
            HardwareAddress::new(0).unwrap(),
            SpiBus::Spi0,
            ChipSelect::Cs0,
            100_000,
            SpiMode::Mode0,
        )
        .expect("Failed to create PFD");
        pfd.init().expect("Failed to initialise PFD");
        assert_eq!(
            pfd.get_mock_data(RegisterAddress::GPINTENB),
            (0b0000_0000, 0, 1)
        );

        {
            let mut pin = pfd.get_input_pin(0).expect("Failed to get pin");
            pin.set_interrupt(InterruptMode::BothEdges)
                .expect("Failed to enable interrupts");
            assert_eq!(
                pfd.get_mock_data(RegisterAddress::GPINTENB),
                (0b0000_0001, 1, 2)
            );
        }
        assert_eq!(
            pfd.get_mock_data(RegisterAddress::GPINTENB),
            (0b0000_0000, 2, 3)
        );
    }

    #[test]
    fn pfd_input_pin_read_levels() {
        let mut pfd = PiFaceDigital::new(
            HardwareAddress::new(0).unwrap(),
            SpiBus::Spi0,
            ChipSelect::Cs0,
            100_000,
            SpiMode::Mode0,
        )
        .expect("Failed to create PFD");
        pfd.init().expect("Failed to initialise PFD");

        let pin = pfd.get_input_pin(0).expect("Failed to get pin");
        assert!(pin.is_low().expect("Bad pin access"));

        pfd.set_mock_data(RegisterAddress::GPIOB, 0b0000_0001);
        assert!(pin.is_high().expect("Bad pin access"));
    }

    #[test]
    fn pfd_init() {
        let mut pfd = PiFaceDigital::new(
            HardwareAddress::new(0).unwrap(),
            SpiBus::Spi0,
            ChipSelect::Cs0,
            100_000,
            SpiMode::Mode0,
        )
        .expect("Failed to create PFD");
        pfd.init().expect("Failed to initialise PFD");

        // Sample a few of the registers for correct values.
        assert_eq!(
            pfd.get_mock_data(RegisterAddress::IODIRA),
            (0x00, 0, 1),
            "Bad IODIRA"
        );
        assert_eq!(
            pfd.get_mock_data(RegisterAddress::IODIRB),
            (0xFF, 0, 1),
            "Bad IODIRB"
        );
        assert_eq!(
            pfd.get_mock_data(RegisterAddress::IOCON),
            (0x28, 1, 1),
            "Bad IOCON"
        );
        assert_eq!(
            pfd.get_mock_data(RegisterAddress::GPPUB),
            (0xFF, 0, 1),
            "Bad GPPUB"
        );
    }

    #[test]
    fn pfd_init_no_hardware() {
        let mut pfd = PiFaceDigital::new(
            HardwareAddress::new(0).unwrap(),
            SpiBus::Spi6, // Magic value that makes mock simulate no hardware.
            ChipSelect::Cs0,
            100_000,
            SpiMode::Mode0,
        )
        .expect("Failed to create PFD");
        let init_result = pfd.init();

        // Check we get the expected error.
        println!("{init_result:?}");
        match init_result {
            Err(PiFaceDigitalError::NoHardwareDetected {
                spi_bus: bus,
                hardware_address: address,
            }) => {
                assert_eq!(bus, SpiBus::Spi6);
                assert_eq!(address, 0.try_into().unwrap())
            }
            _ => panic!("Unexpected return result: {init_result:?}"),
        }
    }

    #[test]
    fn pfd_address_to_mcp23s17_addr() {
        let pfd_addr = HardwareAddress::new(3).expect("valid HardwareAddress");
        let mcp_addr: rppal_mcp23s17::HardwareAddress = pfd_addr.into();
        assert_eq!(
            mcp_addr,
            rppal_mcp23s17::HardwareAddress::new(3).expect("valid HardwareAddress")
        );
    }
    #[test]
    fn good_hardware_address() {
        let addr = HardwareAddress::new(2).expect("Bad address");
        assert_eq!(2u8, addr.into(), "Unexpected address value");
    }

    #[test]
    fn bad_hardware_address() {
        let addr = HardwareAddress::new(4);
        match addr {
            Err(PiFaceDigitalError::HardwareAddressBoundsError(4)) => (),
            _ => panic!("Unexpected return value: {addr:?}"),
        }
    }

    #[test]
    fn try_into_good_hardware_address() {
        let addr: HardwareAddress = 3u8.try_into().expect("Bad address");
        assert_eq!(3u8, addr.into(), "Unexpected address value");
    }

    #[test]
    fn try_into_bad_hardware_address() {
        let addr: Result<HardwareAddress> = 8u8.try_into();
        match addr {
            Err(PiFaceDigitalError::HardwareAddressBoundsError(8)) => (),
            _ => panic!("Unexpected return value: {addr:?}"),
        }
    }
}
