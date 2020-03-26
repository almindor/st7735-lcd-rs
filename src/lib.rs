#![no_std]
// associated re-typing not supported in rust yet
#![allow(clippy::type_complexity)]

//! This crate provides a ST7789 driver to connect to TFT displays.

pub mod instruction;

use crate::instruction::Instruction;
use num_derive::ToPrimitive;
use num_traits::ToPrimitive;

use embedded_hal::blocking::delay::DelayUs;
use embedded_hal::blocking::spi;
use embedded_hal::digital::v2::OutputPin;

#[cfg(feature = "graphics")]
mod graphics;

#[cfg(feature = "batch")]
mod batch;

///
/// ST7789 driver to connect to TFT displays.
///
pub struct ST7789<SPI, CSX, DC, RST>
where
    SPI: spi::Write<u8>,
    CSX: OutputPin,
    DC: OutputPin,
    RST: OutputPin,
{
    // SPI
    spi: SPI,

    // Chip select pin
    csx: CSX,

    // Data/command pin.
    dc: DC,

    // Reset pin.
    rst: RST,

    // Screen size
    size_x: u16,
    size_y: u16,
}

///
/// Display orientation.
///
#[derive(ToPrimitive)]
pub enum Orientation {
    Portrait = 0b0000_0000,         // no inverting
    Landscape = 0b0110_0000,        // invert column and page/column order
    PortraitSwapped = 0b1100_0000,  // invert page and column order
    LandscapeSwapped = 0b1010_0000, // invert page and page/column order
}

///
/// An error holding its source (pins or SPI)
///
#[derive(Debug)]
pub enum Error<SPIE, PinE> {
    Spi(SPIE),
    Pin(PinE),
}

// impl<SPI, CSX, DC, RST, SPIE, PinE> ST7789<SPI, CSX, DC, RST, SPIE, PinE>
impl<SPI, CSX, DC, RST, SPIE, PinE> ST7789<SPI, CSX, DC, RST>
where
    SPI: spi::Write<u8, Error = SPIE>,
    CSX: OutputPin<Error = PinE>,
    DC: OutputPin<Error = PinE>,
    RST: OutputPin<Error = PinE>,
{
    ///
    /// Creates a new ST7789 driver instance
    ///
    /// # Arguments
    ///
    /// * `spi` - an SPI interface to use for talking to the display
    /// * `csx` - the chip select pin documented in the datasheet
    /// * `dc` - data/clock pin switch
    /// * `rst` - display hard reset pin
    /// * `size_x` - x axis resolution of the display in pixels
    /// * `size_y` - y axis resolution of the display in pixels
    ///
    pub fn new(spi: SPI, csx: CSX, dc: DC, rst: RST, size_x: u16, size_y: u16) -> Self {
        Self {
            spi,
            csx,
            dc,
            rst,
            size_x,
            size_y,
        }
    }

    ///
    /// Runs commands to initialize the display
    ///
    pub fn init(&mut self, delay_source: &mut impl DelayUs<u32>) -> Result<(), Error<SPIE, PinE>> {
        self.hard_reset(delay_source)?;
        self.write_command(Instruction::SWRESET, None)?; // reset display
        delay_source.delay_us(150_000);
        self.write_command(Instruction::SLPOUT, None)?; // turn off sleep
        delay_source.delay_us(10_000);
        self.write_command(Instruction::INVOFF, None)?; // turn off invert
        self.write_command(Instruction::MADCTL, Some(&[0b0000_0000]))?; // left -> right, bottom -> top RGB
        self.write_command(Instruction::COLMOD, Some(&[0b0101_0101]))?; // 16bit 65k colors
        self.write_command(Instruction::INVON, None)?; // hack?
        delay_source.delay_us(10_000);
        self.write_command(Instruction::NORON, None)?; // turn on display
        delay_source.delay_us(10_000);
        self.write_command(Instruction::DISPON, None)?; // turn on display
        delay_source.delay_us(10_000);
        Ok(())
    }

    ///
    /// Performs a hard reset using the RST pin sequence
    ///
    pub fn hard_reset(
        &mut self,
        delay_source: &mut impl DelayUs<u32>,
    ) -> Result<(), Error<SPIE, PinE>> {
        self.rst.set_high().map_err(Error::Pin)?;
        delay_source.delay_us(10); // ensure the pin change will get registered
        self.rst.set_low().map_err(Error::Pin)?;
        delay_source.delay_us(10); // ensure the pin change will get registered
        self.rst.set_high().map_err(Error::Pin)?;
        delay_source.delay_us(10); // ensure the pin change will get registered

        Ok(())
    }

    ///
    /// Sets display orientation
    ///
    pub fn set_orientation(&mut self, orientation: &Orientation) -> Result<(), Error<SPIE, PinE>> {
        let orientation = orientation.to_u8().unwrap_or(0);
        self.write_command(Instruction::MADCTL, Some(&[orientation]))?;
        Ok(())
    }

    ///
    /// Sets a pixel color at the given coords.
    ///
    /// # Arguments
    ///
    /// * `x` - x coordinate
    /// * `y` - y coordinate
    /// * `color` - the Rgb565 color value
    ///
    pub fn set_pixel(&mut self, x: u16, y: u16, color: u16) -> Result<(), Error<SPIE, PinE>> {
        self.set_address_window(x, y, x, y)?;
        self.write_command(Instruction::RAMWR, None)?;
        self.start_data()?;
        self.write_word(color)
    }

    ///
    /// Sets pixel colors in given rectangle bounds.
    ///
    /// # Arguments
    ///
    /// * `sx` - x coordinate start
    /// * `sy` - y coordinate start
    /// * `ex` - x coordinate end
    /// * `ey` - y coordinate end
    /// * `colors` - anything that can provide `IntoIterator<Item = u16>` to iterate over pixel data
    ///
    pub fn set_pixels<T>(
        &mut self,
        sx: u16,
        sy: u16,
        ex: u16,
        ey: u16,
        colors: T,
    ) -> Result<(), Error<SPIE, PinE>>
    where
        T: IntoIterator<Item = u16>,
    {
        self.set_address_window(sx, sy, ex, ey)?;
        self.write_command(Instruction::RAMWR, None)?;
        self.start_data()?;

        self.write_pixels(colors)
    }

    #[cfg(not(feature = "buffer"))]
    fn write_pixels<T>(&mut self, colors: T) -> Result<(), Error<SPI, PinE>>
    where
        T: IntoIterator<Item = u16>,
    {
        for color in colors {
            self.write_word(color)?;
        }

        Ok(())
    }

    #[cfg(feature = "buffer")]
    fn write_pixels<T>(&mut self, colors: T) -> Result<(), Error<SPIE, PinE>>
    where
        T: IntoIterator<Item = u16>,
    {
        let mut buf = [0; 128];
        let mut i = 0;

        for color in colors {
            let word = color.to_be_bytes();
            buf[i] = word[0];
            buf[i + 1] = word[1];
            i += 2;

            if i == buf.len() {
                self.write_data(&buf)?;
                i = 0;
            }
        }

        if i > 0 {
            self.write_data(&buf[..i])?;
        }

        Ok(())
    }

    fn write_command(
        &mut self,
        command: Instruction,
        params: Option<&[u8]>,
    ) -> Result<(), Error<SPIE, PinE>> {
        // select this device on the SPI bus
        self.csx.set_low().map_err(Error::Pin)?;
        self.dc.set_low().map_err(Error::Pin)?;

        self.spi
            .write(&[command.to_u8().unwrap()])
            .map_err(Error::Spi)?;

        if let Some(params) = params {
            self.start_data()?;
            self.write_data(params)?;
        }
        // clear chip select when done
        self.csx.set_high().map_err(Error::Pin)?;

        Ok(())
    }

    fn start_data(&mut self) -> Result<(), Error<SPIE, PinE>> {
        self.dc.set_high().map_err(Error::Pin)?;
        Ok(())
    }

    fn write_data(&mut self, data: &[u8]) -> Result<(), Error<SPIE, PinE>> {
        self.csx.set_low().map_err(Error::Pin)?;
        self.spi.write(data).map_err(Error::Spi)?;
        self.csx.set_high().map_err(Error::Pin)?;
        Ok(())
    }

    // Writes a data word to the display.
    fn write_word(&mut self, value: u16) -> Result<(), Error<SPIE, PinE>> {
        self.write_data(&value.to_be_bytes())
    }

    // Sets the address window for the display.
    fn set_address_window(
        &mut self,
        sx: u16,
        sy: u16,
        ex: u16,
        ey: u16,
    ) -> Result<(), Error<SPIE, PinE>> {
        self.write_command(Instruction::CASET, None)?;
        self.start_data()?;
        self.write_word(sx)?;
        self.write_word(ex)?;
        self.write_command(Instruction::RASET, None)?;
        self.start_data()?;
        self.write_word(sy)?;
        self.write_word(ey)
    }
}
