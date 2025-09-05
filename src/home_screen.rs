//! Home screen for a 240x240 TFT using embedded-graphics 0.8 (no_std friendly)

use core::fmt::Write as _;

use embedded_graphics::{
    mono_font::{
        ascii::{FONT_10X20, FONT_6X10, FONT_7X13},
        MonoTextStyle, MonoTextStyleBuilder,
    },
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Circle, PrimitiveStyle, Rectangle, RoundedRectangle},
    text::{Alignment, Baseline, Text},
};
use heapless::String;


use crate::reflow_controller::{ReflowControllerState, Status};
// ---- Colors for the theme ----
#[derive(Copy, Clone)]
pub struct Theme {
    pub bg: Rgb565,
    pub panel: Rgb565,
    pub title: Rgb565,
    pub label: Rgb565,
    pub cur: Rgb565,
    pub set: Rgb565,
    pub unit: Rgb565,
    pub divider: Rgb565,
    pub ok: Rgb565,
    pub warn: Rgb565,
}

use embedded_graphics::pixelcolor::{Rgb888};

#[inline]
fn rgb(hex: u32) -> Rgb565 {
    let r = ((hex >> 16) & 0xFF) as u8;
    let g = ((hex >> 8)  & 0xFF) as u8;
    let b = (hex & 0xFF) as u8;
    Rgb565::from(Rgb888::new(r, g, b))
}

pub fn cobalt2_theme() -> Theme {
    Theme {
        // Deep inky blue background and darker panel blocks
        bg:      rgb(0x193549), // main background
        panel:   rgb(0x12273A), // cards/topbar

        // Text
        title:   rgb(0xFFFFFF), // bright white title
        label:   rgb(0xB8CFE5), // soft desaturated blue for labels
        unit:    rgb(0xE6F8FF), // light cyan for °C

        // Accents
        cur:     rgb(0x00E8FF), // electric cyan (current temp)
        set:     rgb(0xFFC600), // Cobalt2 yellow (set temp)
        divider: rgb(0x102A3C), // subtle line separators

        // Status
        ok:      rgb(0x29D398), // minty green (good)
        warn:    rgb(0xFF4D4D), // alert red (warnings)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            bg: Rgb565::new(6, 6, 6),
            panel: Rgb565::new(12, 12, 12),
            title: Rgb565::WHITE,
            label: Rgb565::new(180, 180, 180),
            cur: Rgb565::CSS_LIGHT_BLUE,
            set: Rgb565::CSS_ORANGE,
            unit: Rgb565::WHITE,
            divider: Rgb565::new(24, 24, 24),
            ok: Rgb565::CSS_DARK_GREEN,
            warn: Rgb565::RED,
        }
    }
}


// Public entry point ----------------------------------------------------------
pub fn draw_home_screen<D: DrawTarget<Color = Rgb565>>(
    target: &mut D,
    size: Size,                       // pass Size::new(240, 240)
    s: &ReflowControllerState,
    profile_name: &str,               // e.g., "Lead-Free"
    theme: Theme,
) -> Result<(), D::Error> {

    // target.clear(theme.bg)?;

    // Background
    Rectangle::new(Point::zero(), size)
        .into_styled(PrimitiveStyle::with_fill(theme.bg))
        .draw(target)?;


    // Temperature band ------------------------------------------------------
    let temp_area = Rectangle::new(Point::new(0, 5), Size::new(size.width, 86));
    draw_temperature_band(target, temp_area, s, theme)?;

    // Info rows: Stage/Ready + Profile + Flags ------------------------------
    let small = MonoTextStyle::new(&FONT_7X13, theme.label);

    // Row 1: Profile
    let mut buf: String<64> = String::new();
    let _ = write!(buf, "Profile: {}", profile_name);
    Text::with_baseline(&buf, Point::new(8, 126), small, Baseline::Top).draw(target)?;


    // Divider above footer
    Rectangle::new(Point::new(0, 200), Size::new(size.width, 1))
        .into_styled(PrimitiveStyle::with_fill(theme.divider))
        .draw(target)?;

    // Footer: D-pad hints (purely informational)
    let hints = MonoTextStyle::new(&FONT_6X10, theme.label);
    Text::with_alignment(
        "↑/↓ Set Temp   → Profiles   ← Manual   ⏺ Start",
        Point::new((size.width / 2) as i32, 208),
        hints,
        Alignment::Center,
    )
    .draw(target)?;

    Ok(())
}

// Temperature band: CUR (left) / SET (right) ---------------------------------
fn draw_temperature_band<D: DrawTarget<Color = Rgb565>>(
    target: &mut D,
    area: Rectangle,                      // e.g., x:0..239, y:34..119
    s: &ReflowControllerState,
    theme: Theme,
) -> Result<(), D::Error> {
    // Split into two halves with a divider
    let mid_x = area.top_left.x + (area.size.width as i32) / 2;
    Rectangle::new(Point::new(mid_x - 1, area.top_left.y), Size::new(2, area.size.height))
        .into_styled(PrimitiveStyle::with_fill(theme.divider))
        .draw(target)?;

    // Shared label style
    let label_style = MonoTextStyle::new(&FONT_6X10, theme.label);

    // Big number style
    let big_cur = MonoTextStyleBuilder::new()
        .font(&FONT_10X20)
        .text_color(theme.cur)
        .build();
    let big_set = MonoTextStyleBuilder::new()
        .font(&FONT_10X20)
        .text_color(theme.set)
        .build();

    // Geometry
    let pad = 10;
    let left = Rectangle::new(
        Point::new(area.top_left.x + pad, area.top_left.y + pad),
        Size::new(area.size.width / 2 - (pad as u32) * 2, area.size.height - (pad as u32) * 2),
    );
    let right = Rectangle::new(
        Point::new(mid_x + pad, area.top_left.y + pad),
        Size::new(area.size.width / 2 - (pad as u32) * 2, area.size.height - (pad as u32) * 2),
    );

    // Draw left: CUR
    Text::with_baseline("Current", left.top_left, label_style, Baseline::Top).draw(target)?;
    draw_temp_value(
        target,
        Point::new(left.top_left.x + 10, left.top_left.y + 14),
        s.current_temperature,
        big_cur,
        theme.unit,
    )?;

    // Draw right: Target
    Text::with_baseline("Target", right.top_left, label_style, Baseline::Top).draw(target)?;
    draw_temp_value(
        target,
        Point::new(right.top_left.x + 10, right.top_left.y + 14),
        s.target_temperature,
        big_set,
        theme.unit,
    )?;

    Ok(())
}

// Draw a temperature like "250°C" with a tiny ° dot (works with ASCII fonts)
fn draw_temp_value<D: DrawTarget<Color = Rgb565>>(
    target: &mut D,
    origin: Point,                         // top-left where digits start
    value_c: f32,
    number_style: MonoTextStyle<Rgb565>,
    unit_color: Rgb565,
) -> Result<(), D::Error> {
    // Compose number
    let mut buf: String<8> = String::new();
    let _ = write!(buf, "{:.1}", value_c);


    // Draw number
    Text::with_baseline(&buf, origin, number_style, Baseline::Top).draw(target)?;

    Ok(())
}
