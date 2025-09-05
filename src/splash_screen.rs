//! Splash screen (no icon) for 240x240 TFT, embedded-graphics 0.8, no_std-friendly

use core::fmt::Write as _;
use heapless::String;

use embedded_graphics::{
    mono_font::ascii::{FONT_6X10, FONT_10X20},
    mono_font::MonoTextStyle,
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::{Alignment, Baseline, Text},
};

use crate::home_screen::Theme;

// Use your Theme + cobalt2_theme() from earlier

fn inset(r: Rectangle, d: i32) -> Rectangle {
    let tl = r.top_left + Point::new(d, d);
    let w = r.size.width.saturating_sub((d as u32) * 2);
    let h = r.size.height.saturating_sub((d as u32) * 2);
    Rectangle::new(tl, Size::new(w, h))
}

pub fn draw_splash_screen<D: DrawTarget<Color = Rgb565>>(
    target: &mut D,
    size: Size,              // Size::new(240, 240)
    version: &str,           // e.g., "v0.1"
    status: &str,            // e.g., "Initializing..."
    progress_pct: u8,        // 0..=100
    theme: Theme,
) -> Result<(), D::Error> {
    // Background
    Rectangle::new(Point::zero(), size)
        .into_styled(PrimitiveStyle::with_fill(theme.bg))
        .draw(target)?;

    // Safe area (avoid edge clipping)
    const SAFE: i32 = 6;
    let screen = inset(Rectangle::new(Point::zero(), size), SAFE);
    let cx = screen.top_left.x + (screen.size.width as i32) / 2;

    // Yellow underline
    Rectangle::new(
        Point::new(screen.top_left.x, screen.top_left.y + 30),
        Size::new(screen.size.width, 3),
    )
    .into_styled(PrimitiveStyle::with_fill(theme.set))
    .draw(target)?;

    // ── Version + status (centered) ────────────────────────────────────────
    let version_style = MonoTextStyle::new(&FONT_10X20, theme.title);
    let mut ver: String<32> = String::new();
    let _ = write!(ver, "Firmware {}", version);
    Text::with_alignment(&ver, Point::new(cx, screen.top_left.y + 68), version_style, Alignment::Center)
        .draw(target)?;

    let status_style = MonoTextStyle::new(&FONT_10X20, theme.cur);
    Text::with_alignment(status, Point::new(cx, screen.top_left.y + 92), status_style, Alignment::Center)
        .draw(target)?;

    // ── Progress bar (bottom) ──────────────────────────────────────────────
    let bar = Rectangle::new(
        Point::new(screen.top_left.x + 8, screen.top_left.y + screen.size.height as i32 - 42),
        Size::new(screen.size.width - 16, 16),
    );

    // Frame/background
    Rectangle::new(bar.top_left, bar.size)
        .into_styled(PrimitiveStyle::with_stroke(theme.divider, 1))
        .draw(target)?;
    Rectangle::new(bar.top_left + Point::new(1, 1), bar.size - Size::new(2, 2))
        .into_styled(PrimitiveStyle::with_fill(theme.panel))
        .draw(target)?;

    // Fill
    let inner = Rectangle::new(bar.top_left + Point::new(1, 1), bar.size - Size::new(2, 2));
    let fill_w = (inner.size.width as u32 * progress_pct as u32) / 100;
    if fill_w > 0 {
        Rectangle::new(inner.top_left, Size::new(fill_w, inner.size.height))
            .into_styled(PrimitiveStyle::with_fill(theme.cur))
            .draw(target)?;
    }

    // Percent (right-aligned above bar)
    let pct_style = MonoTextStyle::new(&FONT_6X10, theme.label);
    let mut pct: String<8> = String::new();
    let _ = write!(pct, "{:>3}%", progress_pct);
    Text::with_alignment(
        &pct,
        Point::new(bar.top_left.x + bar.size.width as i32, bar.top_left.y - 12),
        pct_style,
        Alignment::Right,
    )
    .draw(target)?;

    Ok(())
}
