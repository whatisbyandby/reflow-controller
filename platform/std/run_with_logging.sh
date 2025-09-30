#!/bin/bash

# Script to run reflow controller with serial logging

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "======================================================================"
echo "Reflow Controller with Serial Logging"
echo "======================================================================"
echo

# Check if pyserial is installed
if ! python3 -c "import serial" 2>/dev/null; then
    echo "Error: pyserial is not installed"
    echo "Please install it with: pip3 install pyserial"
    exit 1
fi

# Check if socat is installed
if ! command -v socat &> /dev/null; then
    echo "Error: socat is not installed"
    echo "Please install it with: sudo apt-get install socat"
    exit 1
fi

# Check if binary exists
if [ ! -f "target/debug/reflow-controller-std" ]; then
    echo "Building reflow controller..."
    cargo build
fi

# Create virtual serial port pair
echo "Creating virtual serial ports..."
echo "  /tmp/ttyV0 <-> /tmp/ttyV2 (for commands)"
echo "  /tmp/ttyV1 <-> /tmp/ttyV3 (for data)"
echo

# Clean up old ports
rm -f /tmp/ttyV0 /tmp/ttyV1 /tmp/ttyV2 /tmp/ttyV3

# Start socat for command port (input to controller)
socat pty,raw,echo=0,link=/tmp/ttyV0 pty,raw,echo=0,link=/tmp/ttyV2 &
SOCAT_CMD_PID=$!

# Start socat for data port (output from controller)
socat pty,raw,echo=0,link=/tmp/ttyV1 pty,raw,echo=0,link=/tmp/ttyV3 &
SOCAT_DATA_PID=$!

# Wait for ports to be created
sleep 1

# Verify ports exist
if [ ! -e /tmp/ttyV0 ] || [ ! -e /tmp/ttyV1 ]; then
    echo "Error: Failed to create virtual serial ports"
    kill $SOCAT_CMD_PID $SOCAT_DATA_PID 2>/dev/null || true
    exit 1
fi

echo "✓ Virtual serial ports created"
echo

# Cleanup function
cleanup() {
    echo
    echo "Cleaning up..."
    kill $SOCAT_CMD_PID $SOCAT_DATA_PID 2>/dev/null || true
    kill $CONTROLLER_PID 2>/dev/null || true
    wait $SOCAT_CMD_PID $SOCAT_DATA_PID 2>/dev/null || true
    rm -f /tmp/ttyV0 /tmp/ttyV1 /tmp/ttyV2 /tmp/ttyV3
    echo "Done"
}

trap cleanup EXIT INT TERM

# Start the controller in background
echo "Starting reflow controller..."
./target/debug/reflow-controller-std > controller.log 2>&1 &
CONTROLLER_PID=$!

# Give controller time to start
sleep 2

# Check if controller is still running
if ! kill -0 $CONTROLLER_PID 2>/dev/null; then
    echo "Error: Controller failed to start"
    echo "Check controller.log for details"
    tail controller.log
    exit 1
fi

echo "✓ Controller started (PID: $CONTROLLER_PID)"
echo "  Log: controller.log"
echo

# Parse command line arguments
AUTO_START=""
CSV_FILE="reflow_data.csv"

while [[ $# -gt 0 ]]; do
    case $1 in
        --auto-start)
            AUTO_START="--auto-start"
            shift
            ;;
        -c|--csv)
            CSV_FILE="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Start the Python logger (foreground)
echo "Starting serial logger..."
echo "  Reading from: /tmp/ttyV3 (controller output)"
echo "  Writing to:   /tmp/ttyV2 (controller input)"
echo "  CSV file:     $CSV_FILE"
echo
echo "----------------------------------------------------------------------"
echo

python3 serial_logger.py \
    --output /tmp/ttyV3 \
    --input /tmp/ttyV2 \
    --csv "$CSV_FILE" \
    $AUTO_START

# Cleanup happens automatically via trap