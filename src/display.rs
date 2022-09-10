use std::sync::Arc;

use anyhow::Result;

use embedded_graphics::{pixelcolor::Rgb565, prelude::*, primitives::Rectangle};

use embedded_graphics_framebuf::{AsWords, FrameBuf};
use embedded_hal_0_2::blocking::delay::DelayUs;

use esp_idf_hal::{
    gpio::{Gpio12, Output},
    ledc::{config::TimerConfig, Channel, Timer, CHANNEL0, TIMER0},
    prelude::*,
};
use log::*;
use mipidsi::{Display, DisplayOptions, ColorOrder};

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
    level: u32,
}

impl Backlight {
    pub fn new(channel: CHANNEL0, timer: TIMER0, backlight: Gpio12<Output>) -> Self {
        let config = TimerConfig::default().frequency(5.kHz().into());
        let timer0 =
            Arc::new(Timer::new(timer, &config).expect("Unable to create timer for backlight"));
        let channel = Channel::new(channel, timer0, backlight)
            .expect("Unable to create channel for backlight");
        Self {
            channel,
            level: 100,
        }
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
        let display_options = DisplayOptions {
            color_order: ColorOrder::Bgr,
            ..Default::default()
        };
        self.display
            .init(delay_source, display_options)
            .map_err(|e| {
                info!("Error initializing display {e:?}");
                TwatchError::Display
            })?;
        Ok(())
    }

    pub fn commit_display_partial(&mut self, rect: Rectangle) -> Result<()> {
        if rect.size == self.bounding_box().size {
            self.display
                .write_raw(
                    rect.top_left.x as u16,
                    rect.top_left.y as u16,
                    rect.top_left.x as u16 + rect.size.width as u16,
                    rect.top_left.y as u16 + rect.size.height as u16,
                    self.framebuffer.as_words(),
                )
                .map_err(|_| TwatchError::Display)?;
        } else {
            let mut partial_fb: Vec<u16> = vec![0; (rect.size.width * rect.size.height) as usize];
            let sx: usize = rect.top_left.x as _;
            let ex: usize = (rect.size.width + rect.top_left.x as u32) as _;

            for i in rect.rows() {
                let dsx = (rect.size.width * i as u32) as _;
                let dex = (rect.size.width * (i + 1) as u32) as _;
                let ssx = sx + (i as u32 * 240) as usize;
                let sex = ex + (i as u32 * 240) as usize;
                partial_fb[dsx..dex].copy_from_slice(&self.framebuffer.as_words()[ssx..sex]);
            }

            self.display
                .write_raw(
                    rect.top_left.x as u16,
                    rect.top_left.y as u16,
                    rect.top_left.x as u16 + rect.size.width as u16 - 1,
                    rect.top_left.y as u16 + rect.size.height as u16 - 1,
                    &mut partial_fb,
                )
                .map_err(|_| TwatchError::Display)?;
        }
        Ok(())
    }

    pub fn commit_display(&mut self) -> Result<()> {
        self.commit_display_partial(Rectangle {
            top_left: Point::default(),
            size: Size {
                width: 240,
                height: 240,
            },
        })?;

        self.framebuffer.clear_black();
        Ok(())
    }

    pub fn get_display_level(&self) -> u32 {
        self.backlight.level
    }

    pub fn set_display_level<I: Into<u32>>(&mut self, level: I) -> Result<()> {
        self.backlight.level = level.into();
        let max_duty = self.backlight.channel.get_max_duty();
        self.backlight
            .channel
            .set_duty(self.backlight.level * max_duty / 100)?;
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
