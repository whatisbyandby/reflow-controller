#!/usr/bin/env python3
"""
Serial communication interface for reflow controller
- Reads JSON state messages from serial port
- Logs data to CSV file
- Can send commands to controller
"""

import serial
import json
import csv
import sys
import time
import argparse
from datetime import datetime
import threading
import queue

class ReflowSerialLogger:
    def __init__(self, output_port, input_port=None, csv_file="reflow_data.csv", auto_start=False):
        """
        Args:
            output_port: Serial port to read state data from (e.g., /tmp/ttyV1)
            input_port: Serial port to send commands to (e.g., /tmp/ttyV0), optional
            csv_file: Output CSV filename
            auto_start: Automatically send START command after connection
        """
        self.output_port = output_port
        self.input_port = input_port
        self.csv_file = csv_file
        self.auto_start = auto_start
        self.running = True
        self.ser_output = None
        self.ser_input = None
        self.csv_writer = None
        self.csv_fp = None

    def open_ports(self):
        """Open serial ports"""
        print(f"Opening output port: {self.output_port}")
        try:
            self.ser_output = serial.Serial(
                self.output_port,
                baudrate=115200,
                timeout=1.0
            )
            print(f"✓ Output port opened")
        except Exception as e:
            print(f"Error opening output port: {e}")
            return False

        if self.input_port:
            print(f"Opening input port: {self.input_port}")
            try:
                self.ser_input = serial.Serial(
                    self.input_port,
                    baudrate=115200,
                    timeout=0.1
                )
                print(f"✓ Input port opened")
            except Exception as e:
                print(f"Warning: Could not open input port: {e}")
                print("  Commands will be disabled")

        return True

    def init_csv(self):
        """Initialize CSV file with header"""
        try:
            self.csv_fp = open(self.csv_file, 'w', newline='')
            self.csv_writer = csv.writer(self.csv_fp)
            # Write header
            self.csv_writer.writerow([
                'time_ms', 'status', 'target_temp', 'current_temp',
                'heater_power', 'step', 'profile', 'door_closed', 'fan', 'timer'
            ])
            self.csv_fp.flush()
            print(f"✓ CSV file initialized: {self.csv_file}")
            return True
        except Exception as e:
            print(f"Error creating CSV file: {e}")
            return False

    def send_command(self, command):
        """Send command to controller"""
        if not self.ser_input:
            print("Input port not available")
            return False

        try:
            cmd = f"{command}\n"
            self.ser_input.write(cmd.encode())
            self.ser_input.flush()
            print(f"→ Sent: {command}")
            return True
        except Exception as e:
            print(f"Error sending command: {e}")
            return False

    def read_and_log(self):
        """Main loop: read from serial and log to CSV"""
        print("\nListening for data...")
        print("Press Ctrl+C to stop\n")

        line_buffer = ""
        data_count = 0

        # Auto-start if requested
        if self.auto_start and self.ser_input:
            time.sleep(2)  # Wait for controller to initialize
            self.send_command("START")

        try:
            while self.running:
                try:
                    # Read from serial port
                    if self.ser_output.in_waiting > 0:
                        chunk = self.ser_output.read(self.ser_output.in_waiting)
                        line_buffer += chunk.decode('utf-8', errors='ignore')

                        # Process complete lines
                        while '\n' in line_buffer:
                            line, line_buffer = line_buffer.split('\n', 1)
                            line = line.strip()

                            if line:
                                self.process_line(line)
                                data_count += 1

                                # Show progress every 10 records
                                if data_count % 10 == 0:
                                    print(f"  [{data_count} records logged]", end='\r')

                    else:
                        time.sleep(0.01)

                except KeyboardInterrupt:
                    print("\n\nStopping...")
                    self.running = False
                    break
                except Exception as e:
                    print(f"\nError in read loop: {e}")
                    time.sleep(0.1)

        finally:
            print(f"\nTotal records logged: {data_count}")

    def process_line(self, line):
        """Process a JSON line from serial"""
        try:
            # Parse JSON
            data = json.loads(line)

            # Extract fields
            time_ms = data.get('time_ms', 0)
            state = data.get('state', {})

            # Parse the nested state JSON if it's a string
            if isinstance(state, str):
                state = json.loads(state)

            # Extract state fields
            status = state.get('status', '')
            target_temp = state.get('target_temperature', 0)
            current_temp = state.get('current_temperature', 0)
            heater_power = state.get('heater_power', 0)
            step = state.get('current_step', '')
            profile = state.get('current_profile', '')
            door_closed = state.get('door_closed', False)
            fan = state.get('fan', False)
            timer = state.get('timer', 0)

            # Write to CSV
            self.csv_writer.writerow([
                time_ms, status, f"{target_temp:.2f}", f"{current_temp:.2f}",
                heater_power, step, profile, door_closed, fan, timer
            ])
            self.csv_fp.flush()

            # Print readable status
            print(f"[{time_ms/1000:.1f}s] {status:12} | "
                  f"T: {current_temp:6.1f}°C → {target_temp:6.1f}°C | "
                  f"Power: {heater_power:3}% | "
                  f"Step: {step:12}")

        except json.JSONDecodeError as e:
            print(f"JSON decode error: {e}")
            print(f"  Line: {line[:100]}")
        except Exception as e:
            print(f"Error processing line: {e}")

    def command_loop(self):
        """Interactive command loop in separate thread"""
        if not self.ser_input:
            return

        print("\nCommands: START, STOP, RESET")
        while self.running:
            try:
                cmd = input("> ").strip().upper()
                if cmd:
                    self.send_command(cmd)
            except EOFError:
                break
            except Exception as e:
                if self.running:
                    print(f"Command error: {e}")

    def run(self):
        """Main entry point"""
        print("="*70)
        print("Reflow Controller Serial Logger")
        print("="*70)

        if not self.open_ports():
            return 1

        if not self.init_csv():
            return 1

        # Start command thread if input port is available
        if self.ser_input and not self.auto_start:
            cmd_thread = threading.Thread(target=self.command_loop, daemon=True)
            cmd_thread.start()

        # Main read/log loop
        self.read_and_log()

        # Cleanup
        if self.csv_fp:
            self.csv_fp.close()
        if self.ser_output:
            self.ser_output.close()
        if self.ser_input:
            self.ser_input.close()

        print(f"\n✓ Data saved to: {self.csv_file}")
        return 0

def main():
    parser = argparse.ArgumentParser(
        description='Serial logger for reflow controller',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Read only (no commands)
  %(prog)s -o /tmp/ttyV1

  # Read and send commands
  %(prog)s -o /tmp/ttyV1 -i /tmp/ttyV0

  # Auto-start the reflow process
  %(prog)s -o /tmp/ttyV1 -i /tmp/ttyV0 --auto-start

  # Custom CSV filename
  %(prog)s -o /tmp/ttyV1 -c my_run_001.csv
        """
    )

    parser.add_argument('-o', '--output', required=True,
                        help='Serial port to read data from (e.g., /tmp/ttyV1)')
    parser.add_argument('-i', '--input',
                        help='Serial port to send commands to (e.g., /tmp/ttyV0)')
    parser.add_argument('-c', '--csv', default='reflow_data.csv',
                        help='Output CSV file (default: reflow_data.csv)')
    parser.add_argument('--auto-start', action='store_true',
                        help='Automatically send START command')

    args = parser.parse_args()

    logger = ReflowSerialLogger(
        output_port=args.output,
        input_port=args.input,
        csv_file=args.csv,
        auto_start=args.auto_start
    )

    return logger.run()

if __name__ == "__main__":
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        print("\n\nInterrupted by user")
        sys.exit(1)