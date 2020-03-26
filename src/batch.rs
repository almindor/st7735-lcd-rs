//! Original code from: https://github.com/lupyuen/piet-embedded/blob/master/piet-embedded-graphics/src/batch.rs
//! Batch the pixels to be rendered into Pixel Rows and Pixel Blocks (contiguous Pixel Rows).
//! This enables the pixels to be rendered efficiently as Pixel Blocks, which may be transmitted in a single Non-Blocking SPI request.
use crate::{Error, ST7789};
use embedded_graphics::{
    pixelcolor::{raw::RawU16, Rgb565},
    prelude::*,
};
use embedded_hal::{blocking::spi, digital::v2::OutputPin};

pub trait DrawBatch<SPI, CSX, DC, RST, T, SPIE, PinE>
where
    SPI: spi::Write<u8, Error = SPIE>,
    CSX: OutputPin<Error = PinE>,
    DC: OutputPin<Error = PinE>,
    RST: OutputPin<Error = PinE>,
    T: IntoIterator<Item = Pixel<Rgb565>>,
{
    fn draw_batch(&mut self, item_pixels: T) -> Result<(), Error<SPIE, PinE>>;
}

impl<SPI, CSX, DC, RST, T, SPIE, PinE> DrawBatch<SPI, CSX, DC, RST, T, SPIE, PinE>
    for ST7789<SPI, CSX, DC, RST>
where
    SPI: spi::Write<u8, Error = SPIE>,
    CSX: OutputPin<Error = PinE>,
    DC: OutputPin<Error = PinE>,
    RST: OutputPin<Error = PinE>,
    T: IntoIterator<Item = Pixel<Rgb565>>,
{
    fn draw_batch(&mut self, item_pixels: T) -> Result<(), Error<SPIE, PinE>> {
        //  Get the pixels for the item to be rendered.
        let pixels = item_pixels.into_iter();
        //  Batch the pixels into Pixel Rows.
        let rows = to_rows(pixels);
        //  Batch the Pixel Rows into Pixel Blocks.
        let blocks = to_blocks(rows);
        //  For each Pixel Block...
        for PixelBlock {
            x_left,
            x_right,
            y_top,
            y_bottom,
            colors,
            ..
        } in blocks
        {
            //  Render the Pixel Block.
            self.set_pixels(
                x_left as u16,
                y_top as u16,
                x_right as u16,
                y_bottom as u16,
                colors,
            )?;

            //  Dump out the Pixel Blocks for the square in test_display()
            /* if x_left >= 60 && x_left <= 150 && x_right >= 60 && x_right <= 150 && y_top >= 60 && y_top <= 150 && y_bottom >= 60 && y_bottom <= 150 {
                console::print("pixel block ("); console::printint(x_left as i32); console::print(", "); console::printint(y_top as i32); ////
                console::print("), ("); console::printint(x_right as i32); console::print(", "); console::printint(y_bottom as i32); console::print(")\n"); ////
            } */
        }
        Ok(())
    }
}

/// Max number of pixels per Pixel Row
type MaxRowSize = heapless::consts::U50;
/// Max number of pixels per Pixel Block
type MaxBlockSize = heapless::consts::U100;

/// Consecutive color words for a Pixel Row
type RowColors = heapless::Vec<u16, MaxRowSize>;
/// Consecutive color words for a Pixel Block
type BlockColors = heapless::Vec<u16, MaxBlockSize>;

/// Iterator for each Pixel Row in the pixel data. A Pixel Row consists of contiguous pixels on the same row.
#[derive(Debug, Clone)]
pub struct RowIterator<P: Iterator<Item = Pixel<Rgb565>>> {
    /// Pixels to be batched into rows
    pixels: P,
    /// Start column number
    x_left: u8,
    /// End column number
    x_right: u8,
    /// Row number
    y: u8,
    /// List of pixel colours for the entire row
    colors: RowColors,
    /// True if this is the first pixel for the row
    first_pixel: bool,
}

/// Iterator for each Pixel Block in the pixel data. A Pixel Block consists of contiguous Pixel Rows with the same start and end column number.
#[derive(Debug, Clone)]
pub struct BlockIterator<R: Iterator<Item = PixelRow>> {
    /// Pixel Rows to be batched into blocks
    rows: R,
    /// Start column number
    x_left: u8,
    /// End column number
    x_right: u8,
    /// Start row number
    y_top: u8,
    /// End row number
    y_bottom: u8,
    /// List of pixel colours for the entire block, row by row
    colors: BlockColors,
    /// True if this is the first row for the block
    first_row: bool,
}

/// A row of contiguous pixels
pub struct PixelRow {
    /// Start column number
    pub x_left: u8,
    /// End column number
    pub x_right: u8,
    /// Row number
    pub y: u8,
    /// List of pixel colours for the entire row
    pub colors: RowColors,
}

/// A block of contiguous pixel rows with the same start and end column number
pub struct PixelBlock {
    /// Start column number
    pub x_left: u8,
    /// End column number
    pub x_right: u8,
    /// Start row number
    pub y_top: u8,
    /// End row number
    pub y_bottom: u8,
    /// List of pixel colours for the entire block, row by row
    pub colors: BlockColors,
}

/// Batch the pixels into Pixel Rows, which are contiguous pixels on the same row.
/// P can be any Pixel Iterator (e.g. a rectangle).
fn to_rows<P>(pixels: P) -> RowIterator<P>
where
    P: Iterator<Item = Pixel<Rgb565>>,
{
    RowIterator::<P> {
        pixels,
        x_left: 0,
        x_right: 0,
        y: 0,
        colors: RowColors::new(),
        first_pixel: true,
    }
}

/// Batch the Pixel Rows into Pixel Blocks, which are contiguous Pixel Rows with the same start and end column number
/// R can be any Pixel Row Iterator.
fn to_blocks<R>(rows: R) -> BlockIterator<R>
where
    R: Iterator<Item = PixelRow>,
{
    BlockIterator::<R> {
        rows,
        x_left: 0,
        x_right: 0,
        y_top: 0,
        y_bottom: 0,
        colors: BlockColors::new(),
        first_row: true,
    }
}

/// Implement the Iterator for Pixel Rows.
/// P can be any Pixel Iterator (e.g. a rectangle).
impl<P: Iterator<Item = Pixel<Rgb565>>> Iterator for RowIterator<P> {
    /// This Iterator returns Pixel Rows
    type Item = PixelRow;

    /// Return the next Pixel Row of contiguous pixels on the same row
    fn next(&mut self) -> Option<Self::Item> {
        //  Loop over all pixels until we have composed a Pixel Row, or we have run out of pixels.
        loop {
            //  Get the next pixel.
            let next_pixel = self.pixels.next();
            match next_pixel {
                None => {
                    //  If no more pixels...
                    if self.first_pixel {
                        return None; //  No pixels to group
                    }
                    //  Else return previous pixels as row.
                    let row = PixelRow {
                        x_left: self.x_left,
                        x_right: self.x_right,
                        y: self.y,
                        colors: self.colors.clone(),
                    };
                    self.colors.clear();
                    self.first_pixel = true;
                    return Some(row);
                }
                Some(Pixel(coord, color)) => {
                    //  If there is a pixel...
                    let x = coord.x as u8;
                    let y = coord.y as u8;
                    let color = RawU16::from(color).into_inner();
                    //  Save the first pixel as the row start and handle next pixel.
                    if self.first_pixel {
                        self.first_pixel = false;
                        self.x_left = x;
                        self.x_right = x;
                        self.y = y;
                        self.colors.clear();
                        self.colors.push(color).expect("never");
                        continue;
                    }
                    //  If this pixel is adjacent to the previous pixel, add to the row.
                    if x == self.x_right + 1 && y == self.y && self.colors.push(color).is_ok() {
                        //  Don't add pixel if too many pixels in the row.
                        self.x_right = x;
                        continue;
                    }
                    //  Else return previous pixels as row.
                    let row = PixelRow {
                        x_left: self.x_left,
                        x_right: self.x_right,
                        y: self.y,
                        colors: self.colors.clone(),
                    };
                    self.x_left = x;
                    self.x_right = x;
                    self.y = y;
                    self.colors.clear();
                    self.colors.push(color).expect("never");
                    return Some(row);
                }
            }
        }
    }
}

/// Implement the Iterator for Pixel Blocks.
/// R can be any Pixel Row Iterator.
impl<R: Iterator<Item = PixelRow>> Iterator for BlockIterator<R> {
    /// This Iterator returns Pixel Blocks
    type Item = PixelBlock;

    /// Return the next Pixel Block of contiguous Pixel Rows with the same start and end column number
    fn next(&mut self) -> Option<Self::Item> {
        //  Loop over all Pixel Rows until we have composed a Pixel Block, or we have run out of Pixel Rows.
        loop {
            //  Get the next Pixel Row.
            let next_row = self.rows.next();
            match next_row {
                None => {
                    //  If no more Pixel Rows...
                    if self.first_row {
                        return None; //  No rows to group
                    }
                    //  Else return previous rows as block.
                    let row = PixelBlock {
                        x_left: self.x_left,
                        x_right: self.x_right,
                        y_top: self.y_top,
                        y_bottom: self.y_bottom,
                        colors: self.colors.clone(),
                    };
                    self.colors.clear();
                    self.first_row = true;
                    return Some(row);
                }
                Some(PixelRow {
                    x_left,
                    x_right,
                    y,
                    colors,
                    ..
                }) => {
                    //  If there is a Pixel Row...
                    //  Save the first row as the block start and handle next block.
                    if self.first_row {
                        self.first_row = false;
                        self.x_left = x_left;
                        self.x_right = x_right;
                        self.y_top = y;
                        self.y_bottom = y;
                        self.colors.clear();
                        self.colors.extend_from_slice(&colors).expect("never");
                        continue;
                    }
                    //  If this row is adjacent to the previous row and same size, add to the block.
                    if y == self.y_bottom + 1 && x_left == self.x_left && x_right == self.x_right {
                        //  Don't add row if too many pixels in the block.
                        if self.colors.extend_from_slice(&colors).is_ok() {
                            self.y_bottom = y;
                            continue;
                        }
                    }
                    //  Else return previous rows as block.
                    let row = PixelBlock {
                        x_left: self.x_left,
                        x_right: self.x_right,
                        y_top: self.y_top,
                        y_bottom: self.y_bottom,
                        colors: self.colors.clone(),
                    };
                    self.x_left = x_left;
                    self.x_right = x_right;
                    self.y_top = y;
                    self.y_bottom = y;
                    self.colors.clear();
                    self.colors.extend_from_slice(&colors).expect("never");
                    return Some(row);
                }
            }
        }
    }
}
