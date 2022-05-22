use std::sync::Arc;

use anyhow::Result;

use embedded_graphics::{pixelcolor::Rgb565, prelude::*};

use embedded_graphics_framebuf::{AsWords, FrameBuf};
use embedded_hal_0_2::blocking::delay::DelayUs;

use esp_idf_hal::{
    gpio::{Gpio12, Output},
    ledc::{config::TimerConfig, Channel, Timer, CHANNEL0, TIMER0},
    prelude::*,
};
use mipidsi::Display;

use crate::utils::measure_exec_time;
pub use crate::errors::*;
use crate::types::EspSpi2InterfaceNoCS;

pub struct TwatchDisplay {
    pub display: Display<EspSpi2InterfaceNoCS, mipidsi::NoPin, mipidsi::models::ST7789>,
    pub backlight: Backlight,
    pub framebuffer: &'static mut FrameBuf<Rgb565, 240_usize, 240_usize, 57600_usize>,
}

impl DrawTarget for TwatchDisplay {
    type Color = Rgb565;

    type Error = TwatchError;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        self.framebuffer
            //self.display
            .draw_iter(pixels)
            .map_err(|_| TwatchError::Display)
    }
}

impl OriginDimensions for TwatchDisplay {
    fn size(&self) -> Size {
        Size {
            width: 240,
            height: 240,
        }
    }
}

pub struct Backlight {
    channel: Channel<CHANNEL0, TIMER0, Arc<Timer<TIMER0>>, Gpio12<Output>>,
}

impl Backlight {
    pub fn new(channel: CHANNEL0, timer: TIMER0, backlight: Gpio12<Output>) -> Self {
        let config = TimerConfig::default().frequency(5.kHz().into());
        let timer0 = Arc::new(Timer::new(timer, &config).unwrap());
        let channel = Channel::new(channel, timer0, backlight).unwrap();
        Self { channel }
    }
}

impl TwatchDisplay {
    pub fn new(di: EspSpi2InterfaceNoCS, backlight: Backlight) -> Result<Self> {
        let display = Display::st7789_without_rst(di);
        static mut FBUFF: FrameBuf<Rgb565, 240_usize, 240_usize, 57_600_usize> =
            FrameBuf([Rgb565::BLACK; 57_600]);
        let framebuffer = unsafe { &mut FBUFF };

        Ok(Self {
            display,
            backlight,
            framebuffer,
        })
    }

    pub fn init(&mut self, delay_source: &mut impl DelayUs<u32>) -> Result<()> {
        self.display
            .init(delay_source, Default::default())
            .map_err(|e| {
                log::info!("Error initializing display {:?}", e);
                TwatchError::Display
            })?;
        Ok(())
    }

    pub fn commit_display(&mut self) -> Result<()> {
        measure_exec_time!(
            {
                self.display
                    .write_raw(0, 0, 240, 240, self.framebuffer.as_words())
                    .map_err(|_| TwatchError::Display)?;
            },
            "commit_display"
        );

        self.framebuffer.clear_black();
        Ok(())
    }

    pub fn set_display_level<I: Into<u32>>(&mut self, level: I) -> Result<()> {
        let max_duty = self.backlight.channel.get_max_duty();
        self.backlight
            .channel
            .set_duty(level.into() * max_duty / 100)?;
        Ok(())
    }

    pub fn set_display_on(&mut self) -> Result<()> {
        self.set_display_level(100u32)?;
        Ok(())
    }

    pub fn set_display_off(&mut self) -> Result<()> {
        self.set_display_level(0u32)?;
        Ok(())
    }
}
