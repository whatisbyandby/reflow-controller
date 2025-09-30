# Usage Example

## Complete Workflow: Collect Data and Tune PID

### Step 1: Run Simulation with Auto-Start

```bash
cd /home/scott/Documents/reflow-controller/platform/std
./run_with_logging.sh --auto-start
```

**What happens:**
1. Script creates virtual serial ports (`/tmp/ttyV0` and `/tmp/ttyV1`)
2. Reflow controller starts in background
3. Python logger connects via serial
4. Controller automatically:
   - Closes door after 2 seconds
   - Starts reflow process after 1 more second
5. Real-time data is displayed and logged to CSV
6. Press Ctrl+C when done

**Terminal Output:**
```
======================================================================
Reflow Controller Serial Logger
======================================================================
Opening output port: /tmp/ttyV3
✓ Output port opened
Opening input port: /tmp/ttyV2
✓ Input port opened
✓ CSV file initialized: reflow_data.csv
✓ Controller started (PID: 12345)

Listening for data...
Press Ctrl+C to stop

→ Sent: START
[2.5s] Idle         | T:   25.0°C →   25.0°C | Power:   0% | Step: Preheat
[2.6s] Running      | T:   25.0°C →  150.0°C | Power: 100% | Step: Preheat
[3.5s] Running      | T:   29.5°C →  150.0°C | Power: 100% | Step: Preheat
[4.5s] Running      | T:   38.3°C →  150.0°C | Power: 100% | Step: Preheat
...
```

### Step 2: Analyze Data

```bash
python3 analyze_simple.py
```

**Output:**
```
======================================================================
PID PERFORMANCE ANALYSIS
======================================================================

Current PID Parameters (from code):
  Kp = 3.0
  Ki = 0.5
  Kd = 0.0

----------------------------------------------------------------------
Performance by Step:
----------------------------------------------------------------------

Preheat         Target:  150.0°C
  Duration:         23.0s
  Rise Time:        17.0s
  Overshoot:         0.0°C
  Mean Error:       50.7°C
  Max Error:       125.0°C
  SS Error:          1.3°C
  Oscillations:      0.0

...

======================================================================
RECOMMENDATIONS
======================================================================

Found 2 issue(s):

1. [HIGH] High steady-state error
   Action: Increase Ki from 0.5 to 0.75

2. [MEDIUM] Slow response time
   Action: Increase Kp from 3.0 to 3.9

----------------------------------------------------------------------
SUGGESTED PID VALUES:
----------------------------------------------------------------------

  Kp = 3.9
  Ki = 0.8
  Kd = 0.0

To apply, update src/reflow_controller.rs line 48:
  pid_controller: PidController::new(3.9, 0.8, 0.0),
```

### Step 3: Apply Tuning

Edit `src/reflow_controller.rs`:

```rust
// Line 48 - update these values:
pid_controller: PidController::new(3.9, 0.8, 0.0),
```

Rebuild:
```bash
cargo build
```

### Step 4: Test New Settings

Run another test with updated PID:

```bash
./run_with_logging.sh --auto-start --csv test_run_002.csv
```

Then analyze again:
```bash
python3 analyze_simple.py
```

Compare the results to see if performance improved!

## Manual Control Example

If you want to manually control the process:

```bash
# Terminal 1: Start everything
./run_with_logging.sh

# Wait for "Listening for data..." message
# Then type commands:
> START      # Start the reflow process
> STOP       # Stop if needed
> RESET      # Reset from finished/error state
```

## Multiple Test Runs

To organize multiple tuning iterations:

```bash
# Run 1: Baseline
./run_with_logging.sh --auto-start --csv baseline.csv

# Run 2: After tuning Kp
./run_with_logging.sh --auto-start --csv kp_3.9.csv

# Run 3: After tuning Ki
./run_with_logging.sh --auto-start --csv kp_3.9_ki_0.8.csv

# Compare results
python3 analyze_simple.py    # Analyzes reflow_data.csv (most recent)
```

## Data Format

The CSV file contains time-series data:

```csv
time_ms,status,target_temp,current_temp,heater_power,step,profile,door_closed,fan,timer
1601,Idle,25.00,25.00,0,Preheat,Default Profile,True,False,0
2503,Running,150.00,25.00,100,Preheat,Default Profile,True,False,0
3405,Running,150.00,29.50,100,Preheat,Default Profile,True,False,0
...
```

You can also analyze this data with your own tools (Excel, pandas, etc.)

## Troubleshooting

### Script hangs at "Opening output port"

The controller may not have started yet. Check:
```bash
cat controller.log | grep "Serial port opened"
```

If you see the message, the ports are ready. If not, the controller may have crashed - check the full log.

### Data shows all zeros or -100°C

The temperature sensor simulation needs a moment to initialize. Wait 2-3 seconds after seeing "Idle" status.

### Commands don't work

Make sure you're using the interactive mode (without `--auto-start`), and type commands in UPPERCASE followed by Enter.

### Want to see controller's internal logs

```bash
tail -f controller.log
```

This shows all the internal debug messages from the Rust controller.