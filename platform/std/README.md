# Reflow Controller - STD Platform (Simulation)

This platform runs the reflow controller as a simulation on a standard Linux/macOS system using serial port communication.

## Architecture

The system uses virtual serial ports for communication:

```
┌─────────────────────────┐         ┌──────────────────────────┐
│  Python Logger Script   │         │  Reflow Controller (Rust)│
│                         │         │                          │
│  - Reads state (JSON)   │◄────────│  /tmp/ttyV1 (Output)     │
│  - Writes commands      │────────►│  /tmp/ttyV0 (Input)      │
│  - Logs to CSV          │         │                          │
└─────────────────────────┘         └──────────────────────────┘
```

## Quick Start

### Prerequisites

```bash
# Install required system packages
sudo apt-get install socat

# Install Python dependencies
pip3 install pyserial
```

### Run with Auto-Start

The easiest way to run a complete test:

```bash
./run_with_logging.sh --auto-start
```

This will:
1. Create virtual serial ports
2. Start the reflow controller
3. Auto-close the door and start the reflow process
4. Log all data to `reflow_data.csv`
5. Display real-time status

Press `Ctrl+C` to stop.

### Run with Manual Control

To manually send commands:

```bash
./run_with_logging.sh
```

Then type commands in the terminal:
- `START` - Start reflow process
- `STOP` - Stop current process
- `RESET` - Reset from error/finished state

### Custom CSV Filename

```bash
./run_with_logging.sh --csv test_run_001.csv --auto-start
```

## Manual Usage

If you want more control, you can run components separately:

### 1. Create Virtual Serial Ports

```bash
# Terminal 1: Command port
socat -d -d pty,raw,echo=0,link=/tmp/ttyV0 pty,raw,echo=0,link=/tmp/ttyV2

# Terminal 2: Data port
socat -d -d pty,raw,echo=0,link=/tmp/ttyV1 pty,raw,echo=0,link=/tmp/ttyV3
```

### 2. Build and Run Controller

```bash
# Terminal 3
cargo build
./target/debug/reflow-controller-std
```

### 3. Run Python Logger

```bash
# Terminal 4
python3 serial_logger.py --output /tmp/ttyV3 --input /tmp/ttyV2
```

## Data Analysis

After collecting data, analyze PID performance:

```bash
python3 analyze_simple.py
```

This will:
- Parse the CSV file
- Calculate PID performance metrics (overshoot, rise time, steady-state error)
- Provide tuning recommendations

## Serial Protocol

### Commands (Python → Controller)

Text commands terminated by newline:
- `START\n` - Start reflow process
- `STOP\n` - Stop current process
- `RESET\n` - Reset from error/finished state
- `LIST_PROFILES\n` - Request profile list
- `SET_PROFILE <name>\n` - Load specific profile

### State Data (Controller → Python)

JSON messages, one per line:

```json
{
  "time_ms": 12345,
  "state": {
    "status": "Running",
    "target_temperature": 150.0,
    "current_temperature": 148.5,
    "heater_power": 45,
    "current_step": "Preheat",
    "current_profile": "Default Profile",
    "door_closed": true,
    "fan": false,
    "timer": 120
  }
}
```

## CSV Output Format

Columns:
- `time_ms` - Elapsed milliseconds since start
- `status` - Controller status (Idle/Running/Finished/Error)
- `target_temp` - Target temperature (°C)
- `current_temp` - Current temperature (°C)
- `heater_power` - Heater power (0-100%)
- `step` - Current reflow step name
- `profile` - Profile name
- `door_closed` - Door state (True/False)
- `fan` - Fan state (True/False)
- `timer` - Step timer (seconds)

## Files

- `serial_logger.py` - Python serial interface and CSV logger
- `run_with_logging.sh` - Wrapper script to run everything
- `analyze_simple.py` - PID analysis tool
- `reflow_data.csv` - Output data file (created during run)
- `controller.log` - Controller debug log

## Troubleshooting

### "Could not open serial port"

Make sure socat processes are running and the `/tmp/ttyVX` files exist:

```bash
ls -l /tmp/ttyV*
ps aux | grep socat
```

### "pyserial is not installed"

```bash
pip3 install pyserial
```

### No data in CSV

Check `controller.log` for errors. Make sure the controller started successfully.