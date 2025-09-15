use embassy_time::{Duration, Timer};
use embedded_graphics::{
    mono_font::{ascii::FONT_6X13, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Arc, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, RoundedRectangle},
    text::{Alignment, Baseline, Text, TextStyleBuilder},
};

use heapless::String;

use crate::{
    home_screen::{cobalt2_theme, draw_home_screen, Theme},
    profile::PROFILES,
    reflow_controller::{ReflowControllerState, Status},
    running_screen::{draw_run_screen, RunStage, RunUi},
    splash_screen::draw_splash_screen,
    VERSION,
};

use core::cmp::{max, min};
use core::fmt::Write as _;

use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};

pub static EVENT_CHANNEL: Channel<CriticalSectionRawMutex, Events, 3> = Channel::new();

pub enum Events {
    UpButtonPressed,
    DownButtonPressed,
    RightButtonPressed,
    CenterButtonPressed,
}

fn draw_splash_page<D: DrawTarget<Color = Rgb565>>(display: &mut D) {
    let character_style = MonoTextStyle::new(&FONT_6X13, Rgb565::WHITE);
    let text_style = TextStyleBuilder::new()
        .baseline(Baseline::Middle)
        .alignment(Alignment::Center)
        .build();

    let text = "Reflow Oven Initializing...";

    Text::with_text_style(
        &text,
        display.bounding_box().center(),
        character_style,
        text_style,
    )
    .draw(display)
    .ok();
}

fn format_time_remaining(time_remaining: u32) -> String<6> {
    let minutes = time_remaining / 60;
    let seconds = time_remaining % 60;
    let mut buf: String<6> = String::new();
    let _ = write!(buf, "{:02}:{:02}", minutes, seconds);
    buf
}

fn draw_running_page<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    state: &ReflowControllerState,
) {
    let character_style = MonoTextStyle::new(&FONT_6X13, Rgb565::WHITE);
    let text_style = TextStyleBuilder::new()
        .baseline(Baseline::Middle)
        .alignment(Alignment::Center)
        .build();

    let mut text: String<256> = String::new();
    let _ = write!(
        text,
        "Running {} -> {}\n{}",
        PROFILES[state.current_profile as usize].steps[state.current_step as usize].name,
        format_time_remaining(state.step_time_remaining),
        PROFILES[state.current_profile as usize].name
    );

    Text::with_text_style(
        &text,
        display.bounding_box().center(),
        character_style,
        text_style,
    )
    .draw(display)
    .ok();

    // // Place the widget at y=92..112 from your earlier plan
    // let widget_area = Rectangle::new(Point::new(8, 92), Size::new(304, 36));

    // let colors = HeaterBarColors {
    //     label: Rgb565::YELLOW,          // "Heater"
    //     frame: Rgb565::new(8, 8, 8),    // subtle dark frame
    //     bg: Rgb565::new(20, 20, 20),    // bar background
    //     fill: Rgb565::new(255, 140, 0), // orange fill
    //     pct: Rgb565::WHITE,
    // };

    // draw_heater_bar(display, widget_area, "Heater", state.heater_power, colors).ok();
}

fn draw_error_page<D: DrawTarget<Color = Rgb565>>(display: &mut D, state: &ReflowControllerState) {
    let character_style = MonoTextStyle::new(&FONT_6X13, Rgb565::WHITE);
    let text_style = TextStyleBuilder::new()
        .baseline(Baseline::Middle)
        .alignment(Alignment::Center)
        .build();

    let text = "Reflow Oven Error";

    Text::with_text_style(
        &text,
        display.bounding_box().center(),
        character_style,
        text_style,
    )
    .draw(display)
    .ok();
}

fn get_progress(total_time: u32, elapsed_time: u32) -> u8 {
    if total_time == 0 {
        0
    } else {
        (elapsed_time * 100 / total_time) as u8
    }
}

pub fn draw_page<D: DrawTarget<Color = Rgb565>>(display: &mut D, state: &ReflowControllerState) {
    let theme = cobalt2_theme();
    match state.status {
        Status::Initializing => {
            draw_splash_screen(
                display,
                Size::new(240, 240),
                VERSION,
                // if less than 50 percent show initalizing, if greater show almost ready,
                "Initializing...",
                50,
                theme,
            )
            .ok();
            return;
        }
        Status::Idle => {
            draw_home_screen(
                display,
                Size::new(240, 240),
                &state,
                PROFILES[state.current_profile as usize].name,
                theme,
            )
            .ok();
            return;
        }
        Status::Running => {
            let ui = RunUi {
                stage: RunStage::Reflow,
                progress_pct: get_progress(
                    PROFILES[state.current_profile as usize].steps[state.current_step as usize]
                        .time,
                    PROFILES[state.current_profile as usize].steps[state.current_step as usize]
                        .time
                        - state.step_time_remaining,
                ),
                time_left_s: state.step_time_remaining,
                paused: false,
            };
            draw_run_screen(display, Size::new(240, 240), &state, &ui, cobalt2_theme()).ok();
            return;
        }
        Status::Error => {
            draw_error_page(display, state);
            return;
        }
    }
}
