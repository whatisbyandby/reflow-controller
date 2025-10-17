#[path = "./serial_port.rs"]
mod serial_port;

use crate::text_interface::{handle_command, to_json_heapless, ProfileListResponse, ActiveProfileResponse};
use crate::{
    ACTIVE_PROFILE_CHANNEL, CURRENT_STATE, PROFILE_LIST_CHANNEL, SYSTEM_TICK_MILLIS,
};
use embassy_executor::Spawner;
use embassy_time::Timer;

use async_io::Async;
use embedded_io_async::{Read, Write as AsyncWrite};
use nix::sys::termios;

use self::serial_port::SerialPort;

async fn handle_serial_data(data: &[u8]) {
    handle_command(data);
}

#[embassy_executor::task]
async fn serial_reader_task(serial_path: &'static str) {
    let baudrate = termios::BaudRate::B115200;

    // Try to open serial port, but continue if it fails
    let port_result = SerialPort::new(serial_path, baudrate);
    if port_result.is_err() {
        log::warn!("Could not open {} - serial input disabled", serial_path);
        log::info!("To enable serial, create virtual ports with:");
        log::info!("  socat -d -d pty,raw,echo=0,link=/tmp/ttyV0 pty,raw,echo=0,link={}", serial_path);
        loop {
            Timer::after_millis(10000).await;
        }
    }

    let port = port_result.unwrap();
    let port = Async::new(port).unwrap();
    let mut port = embedded_io_adapters::futures_03::FromFutures::new(port);

    log::info!("Serial port opened for reading: {}", serial_path);

    // Line buffer to accumulate data until we receive a newline
    let mut line_buffer = heapless::Vec::<u8, 4096>::new();

    loop {
        let mut buf = [0u8; 256];
        match port.read(&mut buf).await {
            Ok(n) if n > 0 => {
                // Process each byte, looking for newlines
                for &byte in &buf[..n] {
                    if byte == b'\n' || byte == b'\r' {
                        // End of line - process the command if buffer has data
                        if !line_buffer.is_empty() {
                            handle_serial_data(&line_buffer).await;
                            line_buffer.clear();
                        }
                    } else {
                        // Add byte to buffer if there's space
                        if line_buffer.push(byte).is_err() {
                            log::warn!("Command too long, discarding buffer");
                            line_buffer.clear();
                        }
                    }
                }
            }
            Ok(_) => {}
            Err(e) => {
                log::error!("Serial read error: {:?}", e);
                Timer::after_millis(1000).await;
            }
        }
    }
}

#[embassy_executor::task]
async fn profile_list_task() {
    let receiver = PROFILE_LIST_CHANNEL.receiver();
    loop {
        let profiles = receiver.receive().await;
        let response = ProfileListResponse { profiles };
        let json: heapless::String<1024> = serde_json_core::ser::to_string(&response).unwrap();
        log::info!("{}", json);
    }
}

#[embassy_executor::task]
async fn active_profile_task() {
    let receiver = ACTIVE_PROFILE_CHANNEL.receiver();
    loop {
        let profile = receiver.receive().await;
        let response = ActiveProfileResponse {
            active_profile: profile,
        };
        let json: heapless::String<2048> = serde_json_core::ser::to_string(&response).unwrap();
        log::info!("{}", json);
    }
}

#[embassy_executor::task]
async fn serial_writer_task(serial_path: &'static str) {
    let baudrate = termios::BaudRate::B115200;

    // Wait a bit for reader to open first
    Timer::after_millis(500).await;

    // Open serial port for writing
    let port_result = SerialPort::new(serial_path, baudrate);
    if port_result.is_err() {
        log::warn!("Could not open {} for writing - serial output disabled", serial_path);
        loop {
            Timer::after_millis(10000).await;
        }
    }

    let port = port_result.unwrap();
    let port = Async::new(port).unwrap();
    let mut port = embedded_io_adapters::futures_03::FromFutures::new(port);

    log::info!("Serial port opened for writing: {}", serial_path);

    let mut receiver = CURRENT_STATE.receiver().unwrap();
    let start_time = std::time::Instant::now();

    loop {
        let new_state = receiver.get().await;
        let elapsed_ms = start_time.elapsed().as_millis();

        // Create JSON with timestamp
        let json = to_json_heapless(&new_state);

        // Add timestamp and send to serial (one line per message)
        let mut output = heapless::String::<1200>::new();
        use core::fmt::Write;
        write!(&mut output, "{{\"time_ms\":{},\"state\":{}}}\n", elapsed_ms, json.as_str()).ok();

        // Write to serial port
        match port.write_all(output.as_bytes()).await {
            Ok(_) => {
                // Flush to ensure data is sent
                match port.flush().await {
                    Ok(_) => {},
                    Err(e) => log::error!("Serial flush error: {:?}", e),
                }
            }
            Err(e) => {
                log::error!("Serial write error: {:?}", e);
            }
        }

        Timer::after_millis(SYSTEM_TICK_MILLIS.into()).await;
    }
}

#[embassy_executor::task]
pub async fn usb_task(spawner: Spawner) {
    // Use separate serial ports: one for input (commands), one for output (state)
    const SERIAL_INPUT: &'static str = "/tmp/ttyV0";  // Python writes commands here
    const SERIAL_OUTPUT: &'static str = "/tmp/ttyV1"; // Python reads state from here

    spawner.spawn(serial_reader_task(SERIAL_INPUT).unwrap());
    spawner.spawn(serial_writer_task(SERIAL_OUTPUT).unwrap());
    // spawner.spawn(profile_list_task().unwrap());
    // spawner.spawn(active_profile_task().unwrap());

    log::info!("Serial interface initialized: input={}, output={}", SERIAL_INPUT, SERIAL_OUTPUT);

    // Keep the main task alive
    loop {
        Timer::after_millis(10000).await;
    }
}
