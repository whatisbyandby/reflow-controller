//! Running screen (240x240), Cobalt2-style, no icons, no_std-friendly.
//! embedded-graphics = "0.8", heapless = "0.8"

use core::fmt::Write as _;
use heapless::String;

use embedded_graphics::{
    mono_font::{
        ascii::{FONT_6X10, FONT_10X20},
        MonoTextStyle, MonoTextStyleBuilder,
    },
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Circle, PrimitiveStyle, Rectangle, RoundedRectangle},
    text::{Alignment, Baseline, Text},
};

use crate::{home_screen::Theme, profile::PROFILES, reflow_controller::ReflowControllerState};


#[derive(Copy, Clone)]
pub enum RunStage { Preheat, Soak, Reflow, Cool }

pub struct RunUi {
    pub stage: RunStage,
    pub progress_pct: u8, // 0..=100
    pub time_left_s: u32, // seconds
    pub paused: bool,
}

// ────────────────────────────────────────────────────────────────────────────
// Public entry point
// ────────────────────────────────────────────────────────────────────────────
pub fn draw_run_screen<D: DrawTarget<Color = Rgb565>>(
    target: &mut D,
    size: Size,                 // Size::new(240, 240)
    s: &ReflowControllerState,
    ui: &RunUi,
    theme: Theme,
) -> Result<(), D::Error> {
    // 0) Clear and use a safe inset to avoid edge clipping
    Rectangle::new(Point::zero(), size)
        .into_styled(PrimitiveStyle::with_fill(theme.bg))
        .draw(target)?;
    const SAFE: i32 = 6;
    let screen = inset(Rectangle::new(Point::zero(), size), SAFE);


    // 2) Temps row: CUR left / SET right
    let temps_area = Rectangle::new(
        Point::new(screen.top_left.x, screen.top_left.y + 32),
        Size::new(screen.size.width, 70),
    );
    draw_temps_row(target, temps_area, s, theme)?;

    // 3) Progress bar + time left
    let prog_area = Rectangle::new(
        Point::new(screen.top_left.x + 6, screen.top_left.y + 110),
        Size::new(screen.size.width - 12, 36),
    );
    draw_labeled_bar(
        target,
        prog_area,
        PROFILES[s.current_profile as usize].steps[s.current_step as usize].name,
        ui.progress_pct,
        theme.label,          // label color
        theme.divider,        // frame line
        theme.ok,             // fill (mint green)
        theme.panel,          // bar background
    )?;

    let small = MonoTextStyle::new(&FONT_6X10, theme.label);
    let mut tleft: String<16> = String::new();
    let _ = write!(tleft, "{:02}:{:02} left", ui.time_left_s / 60, ui.time_left_s % 60);
    Text::with_alignment(
        &tleft,
        Point::new(prog_area.top_left.x + prog_area.size.width as i32, prog_area.top_left.y - 12),
        small,
        Alignment::Right,
    );

    Ok(())
}

// ────────────────────────────────────────────────────────────────────────────
// Helpers (all safe-area aware, ASCII fonts only)
// ────────────────────────────────────────────────────────────────────────────

fn inset(r: Rectangle, d: i32) -> Rectangle {
    let tl = r.top_left + Point::new(d, d);
    let w = r.size.width.saturating_sub((d as u32) * 2);
    let h = r.size.height.saturating_sub((d as u32) * 2);
    Rectangle::new(tl, Size::new(w, h))
}

fn draw_temps_row<D: DrawTarget<Color = Rgb565>>(
    target: &mut D,
    area: Rectangle,
    s: &ReflowControllerState,
    theme: Theme,
) -> Result<(), D::Error> {
    let mid_x = area.top_left.x + (area.size.width as i32) / 2;
    Rectangle::new(Point::new(mid_x - 1, area.top_left.y), Size::new(2, area.size.height))
        .into_styled(PrimitiveStyle::with_fill(theme.divider)).draw(target)?;

    let label = MonoTextStyle::new(&FONT_6X10, theme.label);
    let big_cur = MonoTextStyleBuilder::new().font(&FONT_10X20).text_color(theme.cur).build();
    let big_set = MonoTextStyleBuilder::new().font(&FONT_10X20).text_color(theme.set).build();

    let pad = 8;
    let left = Rectangle::new(
        Point::new(area.top_left.x + pad, area.top_left.y + pad),
        Size::new(area.size.width/2 - (pad as u32)*2, area.size.height - (pad as u32)*2),
    );
    let right = Rectangle::new(
        Point::new(mid_x + pad, area.top_left.y + pad),
        Size::new(area.size.width/2 - (pad as u32)*2, area.size.height - (pad as u32)*2),
    );

    Text::with_baseline("Current", left.top_left, label, Baseline::Top).draw(target)?;
    draw_temp_value(target, Point::new(left.top_left.x, left.top_left.y + 14),
                    s.current_temperature, big_cur, theme.unit)?;

    Text::with_baseline("Target", right.top_left, label, Baseline::Top).draw(target)?;
    draw_temp_value(target, Point::new(right.top_left.x, right.top_left.y + 14),
                    s.target_temperature, big_set, theme.unit)?;
    Ok(())
}

fn draw_temp_value<D: DrawTarget<Color = Rgb565>>(
    target: &mut D,
    origin: Point,
    val_c: f32,
    number_style: MonoTextStyle<Rgb565>,
    unit_color: Rgb565,
) -> Result<(), D::Error> {
    let mut buf: String<8> = String::new();
    let _ = write!(buf, "{:.1}", val_c);
    Text::with_baseline(&buf, origin, number_style, Baseline::Top).draw(target)?;
    Ok(())
}

fn draw_labeled_bar<D: DrawTarget<Color = Rgb565>>(
    target: &mut D,
    area: Rectangle,
    label: &str,
    pct: u8,
    label_color: Rgb565,
    frame_color: Rgb565,
    fill_color: Rgb565,
    bg_color: Rgb565,
) -> Result<(), D::Error> {
    let label_style = MonoTextStyle::new(&FONT_6X10, label_color);
    Text::with_baseline(label, area.top_left, label_style, Baseline::Top).draw(target)?;

    // bar rect
    let bar = Rectangle::new(
        Point::new(area.top_left.x, area.top_left.y + 12),
        Size::new(area.size.width, area.size.height.saturating_sub(14)),
    );
    // frame + bg
    Rectangle::new(bar.top_left, bar.size)
        .into_styled(PrimitiveStyle::with_stroke(frame_color, 1))
        .draw(target)?;
    Rectangle::new(bar.top_left + Point::new(1,1), bar.size - Size::new(2,2))
        .into_styled(PrimitiveStyle::with_fill(bg_color))
        .draw(target)?;
    // fill
    let inner = Rectangle::new(bar.top_left + Point::new(1,1), bar.size - Size::new(2,2));
    let fill_w = (inner.size.width as u32 * pct as u32) / 100;
    if fill_w > 0 {
        Rectangle::new(inner.top_left, Size::new(fill_w, inner.size.height))
            .into_styled(PrimitiveStyle::with_fill(fill_color))
            .draw(target)?;
    }
    // percent (right-aligned, above bar)
    let pct_style = MonoTextStyle::new(&FONT_6X10, label_color);
    let mut buf: String<8> = String::new();
    let _ = write!(buf, "{:>3}%", pct);
    Text::with_alignment(
        &buf,
        Point::new(area.top_left.x + area.size.width as i32, area.top_left.y),
        pct_style,
        Alignment::Right,
    );

    Ok(())
}
