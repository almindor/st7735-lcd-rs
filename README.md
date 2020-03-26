# st7789

This is a Rust driver library for ST7789 displays using embedded_graphics, embedded_hal, and no_std, no_alloc support. 
- [Driver documentation](https://docs.rs/st7789). 
- [Examples](https://github.com/almindor/st7789-examples)
- [Display datasheet](https://www.rhydolabz.com/documents/33/ST7789.pdf)

[![ferris-demo](http://objdump.katona.me/ferris_fast.png)](http://objdump.katona.me/ferris_fast.mp4)

## Features

These features are enabled by default:

* `graphics` - embedded-graphics support: pulls in [embedded-graphics](https://crates.io/crates/embedded-graphics) dependency
* `batch` - batch-drawing optimization: pulls in [heapless](https://crates.io/crates/heapless) dependency and allocates 300 bytes for frame buffer in the driver
* `buffer` - use a 128 byte buffer for SPI data transfers

## Status

- [x] Communications via SPI
- [x] Tested with PineTime watch
- [ ] Offscreen Buffering
- [ ] Hardware scrolling support

## Changelog

* `v0.2.2` - add buffering for SPI transfers
* `v0.2.1` - use static dispatch for `set_pixels`
* `v0.2.0` - batch support
* `v0.1.0` - initial release

