use anyhow::Result;

use embedded_graphics::{pixelcolor::Rgb565, prelude::*};

use embedded_graphics_framebuf::FrameBuf;

use embedded_hal_0_2::blocking::delay::DelayUs;
use embedded_hal_0_2::digital::v2::OutputPin;
use esp_idf_hal::gpio::{self, Output};
use mipidsi::Display;

pub use crate::errors::*;
use crate::types::EspSpi2InterfaceNoCS;

pub struct TwatchDisplay<'a> {
    pub display: Display<EspSpi2InterfaceNoCS, mipidsi::NoPin, mipidsi::models::ST7789>,
    pub frame_buffer: &'a mut FrameBuf<Rgb565, 240_usize, 240_usize>,
    pub backlight: gpio::Gpio12<Output>,
}

impl<'a> DrawTarget for TwatchDisplay<'a> {
    type Color = Rgb565;

    type Error = TwatchError;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        self.frame_buffer
           .draw_iter(pixels)
           .map_err(|_| TwatchError::Display)
    }
}

impl<'a> OriginDimensions for TwatchDisplay<'a> {
    fn size(&self) -> Size {
        Size {
            width: 240,
            height: 240,
        }
    }
}

impl TwatchDisplay<'static> {
    pub fn new(di: EspSpi2InterfaceNoCS, backlight: gpio::Gpio12<Output>) -> Result<Self> {
        static mut FBUFF: FrameBuf<Rgb565, 240_usize, 240_usize> =
            FrameBuf([[embedded_graphics::pixelcolor::Rgb565::BLACK; 240]; 240]);
        let frame_buffer = unsafe { &mut FBUFF };

        let display = Display::st7789_without_rst(di);

        Ok(Self {
            display,
            frame_buffer,
            backlight,
        })
    }

    pub fn init(&mut self, delay_source: &mut impl DelayUs<u32>) -> Result<()> {
        self.set_display_on()?;
        self.display
            .init(delay_source, Default::default())
            .map_err(|_| TwatchError::Display)?;
        Ok(())
    }

    pub fn commit_display(&mut self) -> Result<()> {
        self.display
            .set_pixels(0, 0, 240, 240, self.frame_buffer.into_iter())
            .map_err(|_| TwatchError::Display)?;
        self.frame_buffer.clear_black();
        Ok(())
    }

    pub fn set_display_on(&mut self) -> Result<()> {
        self.backlight.set_high()?;
        Ok(())
    }

    pub fn set_display_off(&mut self) -> Result<()> {
        self.backlight.set_low()?;
        Ok(())
    }
}
