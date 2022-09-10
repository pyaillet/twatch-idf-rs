use std::cmp::{max, min};

use embedded_graphics::{
    mono_font::MonoTextStyle,
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyleBuilder, Rectangle},
    text::Text,
    Drawable,
};
use ft6x36::{Direction, TouchEvent};
use profont::PROFONT_18_POINT;

use log::*;
use u8g2_fonts::{
    fonts,
    types::{FontColor, HorizontalAlignment, VerticalPosition},
    FontRenderer,
};

use crate::{
    events::{Kind, TwatchEvent},
    tiles::WatchTile,
};

#[derive(Default)]
pub struct LightTile {}

unsafe impl Send for LightTile {}

impl WatchTile for LightTile {
    fn run(&mut self, hal: &mut crate::twatch::Hal<'static>) -> anyhow::Result<()> {
        self.display_tile(hal)?;
        hal.display.commit_display()?;

        Ok(())
    }

    fn display_tile(&self, hal: &mut crate::twatch::Hal<'static>) -> anyhow::Result<()> {
        let style = MonoTextStyle::new(&PROFONT_18_POINT, Rgb565::WHITE);
        let font = FontRenderer::new::<fonts::u8g2_font_logisoso92_tn>();

        let level = format!("Light level: {}", hal.display.get_display_level());
        Text::new(&level, Point::new(0, 30), style).draw(&mut hal.display)?;

        let rect_style = PrimitiveStyleBuilder::new()
            .stroke_width(2)
            .stroke_color(Rgb565::BLUE)
            .build();

        Rectangle::new(Point::new(20, 80), Size::new(90, 90))
            .into_styled(rect_style)
            .draw(&mut hal.display)?;

        font.render_aligned(
            "-",
            Point::new(65, 160),
            VerticalPosition::Baseline,
            HorizontalAlignment::Center,
            FontColor::Transparent(Rgb565::WHITE),
            &mut hal.display,
        )
        .expect("-");

        Rectangle::new(Point::new(130, 80), Size::new(90, 90))
            .into_styled(rect_style)
            .draw(&mut hal.display)?;

        font.render_aligned(
            "+",
            Point::new(175, 160),
            VerticalPosition::Baseline,
            HorizontalAlignment::Center,
            FontColor::Transparent(Rgb565::WHITE),
            &mut hal.display,
        )
        .expect("+");

        Ok(())
    }

    fn process_event(
        &mut self,
        hal: &mut crate::twatch::Hal<'static>,
        event: crate::events::TwatchEvent,
    ) -> Option<crate::events::TwatchEvent> {
        match (&event.time, &event.kind) {
            (_, Kind::Touch(TouchEvent::Swipe(dir, _info))) => match dir {
                Direction::Left => {
                    let mut hello_tile = crate::tiles::hello::HelloTile::default();
                    let _ = crate::tiles::move_to_tile(hal, self, &mut hello_tile, dir);
                    Some(TwatchEvent::new(Kind::NewTile(Box::new(hello_tile))))
                }
                Direction::Right => {
                    let mut motor_tile = crate::tiles::motor::MotorTile::default();
                    let _ = crate::tiles::move_to_tile(hal, self, &mut motor_tile, dir);
                    Some(TwatchEvent::new(Kind::NewTile(Box::new(motor_tile))))
                }
                _ => {
                    info!("Swipe: {dir:?}");
                    None
                }
            },
            (_, Kind::Touch(TouchEvent::TouchOnePoint(p))) => {
                if p.y >= 80 && p.y <= 170 {
                    let mut level = hal.display.get_display_level();
                    if p.x <= 120 {
                        level = min(100, level + 15);
                    } else {
                        level = max(10, level.saturating_sub(15));
                    }
                    hal.display
                        .set_display_level(level)
                        .unwrap_or_else(|e| warn!("Unable to change backlight level: {e:?}"));
                    self.display_tile(hal).expect("Unable to update tile");
                    hal.display.commit_display().expect("Unable to commit fb");

                    None
                } else {
                    Some(event)
                }
            }
            _ => Some(event),
        }
    }
}
