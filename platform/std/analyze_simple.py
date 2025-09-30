#!/usr/bin/env python3
"""
Simple PID analysis without external dependencies
"""

import csv
from collections import defaultdict

def load_csv(filename="reflow_data.csv"):
    """Load CSV data"""
    data = []
    with open(filename, 'r') as f:
        reader = csv.DictReader(f)
        for row in reader:
            data.append({
                'time_s': float(row['time_ms']) / 1000.0,
                'status': row['status'],
                'target_temp': float(row['target_temp']),
                'current_temp': float(row['current_temp']),
                'heater_power': int(row['heater_power']),
                'step': row['step'],
                'door_closed': row['door_closed'] == 'true',
                'fan': row['fan'] == 'true'
            })
    return data

def analyze_step(step_data, step_name):
    """Analyze performance for a single step"""
    if not step_data:
        return None

    # Filter only Running status
    running = [d for d in step_data if d['status'] == 'Running']
    if not running:
        return None

    target = running[0]['target_temp']
    start_time = running[0]['time_s']
    end_time = running[-1]['time_s']
    start_temp = running[0]['current_temp']

    # Calculate metrics
    errors = [d['target_temp'] - d['current_temp'] for d in running]
    temps = [d['current_temp'] for d in running]

    mean_error = sum(errors) / len(errors)
    max_error = max(abs(e) for e in errors)
    ss_error = sum(errors[-10:]) / min(10, len(errors))

    # Find overshoot
    is_cooling = 'Cool' in step_name or step_name == 'Cooling'
    overshoot = 0
    if not is_cooling:
        max_temp = max(temps)
        if max_temp > target:
            overshoot = max_temp - target

    # Rise time (time to 90% of target)
    rise_time = 0
    ninety_percent = start_temp + 0.9 * (target - start_temp)
    for d in running:
        if not is_cooling and d['current_temp'] >= ninety_percent:
            rise_time = d['time_s'] - start_time
            break
        elif is_cooling and d['current_temp'] <= ninety_percent:
            rise_time = d['time_s'] - start_time
            break

    # Count oscillations
    sign_changes = 0
    for i in range(1, len(errors)):
        if (errors[i-1] > 0) != (errors[i] > 0):
            sign_changes += 1
    oscillations = sign_changes / 2.0

    return {
        'step_name': step_name,
        'target': target,
        'duration': end_time - start_time,
        'mean_error': mean_error,
        'max_error': max_error,
        'ss_error': ss_error,
        'overshoot': overshoot,
        'rise_time': rise_time,
        'oscillations': oscillations
    }

def main():
    print("Loading CSV data...")
    data = load_csv()

    print(f"Loaded {len(data)} data points")
    print(f"Time range: {data[0]['time_s']:.1f}s to {data[-1]['time_s']:.1f}s")

    # Group by step
    steps = defaultdict(list)
    for d in data:
        steps[d['step']].append(d)

    print("\n" + "="*70)
    print("PID PERFORMANCE ANALYSIS")
    print("="*70)

    print(f"\nCurrent PID Parameters (from code):")
    print(f"  Kp = 3.0")
    print(f"  Ki = 0.5")
    print(f"  Kd = 0.0")

    all_metrics = []
    for step_name in sorted(steps.keys()):
        metrics = analyze_step(steps[step_name], step_name)
        if metrics:
            all_metrics.append(metrics)

    print("\n" + "-"*70)
    print("Performance by Step:")
    print("-"*70)

    for m in all_metrics:
        print(f"\n{m['step_name']:15} Target: {m['target']:6.1f}°C")
        print(f"  Duration:       {m['duration']:6.1f}s")
        print(f"  Rise Time:      {m['rise_time']:6.1f}s")
        print(f"  Overshoot:      {m['overshoot']:6.1f}°C")
        print(f"  Mean Error:     {m['mean_error']:6.1f}°C")
        print(f"  Max Error:      {m['max_error']:6.1f}°C")
        print(f"  SS Error:       {m['ss_error']:6.1f}°C")
        print(f"  Oscillations:   {m['oscillations']:6.1f}")

    # Analyze overall performance
    print("\n" + "="*70)
    print("RECOMMENDATIONS")
    print("="*70)

    avg_overshoot = sum(m['overshoot'] for m in all_metrics) / len(all_metrics)
    avg_oscillations = sum(m['oscillations'] for m in all_metrics) / len(all_metrics)
    max_ss_error = max(abs(m['ss_error']) for m in all_metrics)

    recommendations = []

    if max_ss_error > 5:
        recommendations.append({
            'priority': 'HIGH',
            'issue': 'High steady-state error',
            'action': 'Increase Ki from 0.5 to 0.75'
        })

    if avg_overshoot > 10:
        recommendations.append({
            'priority': 'HIGH',
            'issue': 'Excessive overshoot',
            'action': 'Decrease Kp from 3.0 to 2.4'
        })
    elif avg_overshoot > 5:
        recommendations.append({
            'priority': 'MEDIUM',
            'issue': 'Moderate overshoot detected',
            'action': 'Add derivative term: Kd = 0.3'
        })

    if avg_oscillations > 3:
        recommendations.append({
            'priority': 'HIGH',
            'issue': 'System oscillating',
            'action': 'Reduce Kp to 2.1 and Ki to 0.4'
        })

    # Check for slow response
    slow_steps = sum(1 for m in all_metrics if m['rise_time'] > m['duration'] * 0.6)
    if slow_steps > len(all_metrics) / 2:
        recommendations.append({
            'priority': 'MEDIUM',
            'issue': 'Slow response time',
            'action': 'Increase Kp from 3.0 to 3.9'
        })

    if not recommendations:
        print("\n✓ PID controller is performing well!")
        print("\nOptional fine-tuning:")
        print("  - For faster response: Kp = 3.3")
        print("  - For less overshoot: Kp = 2.7")
    else:
        print(f"\nFound {len(recommendations)} issue(s):\n")
        for i, rec in enumerate(recommendations, 1):
            print(f"{i}. [{rec['priority']}] {rec['issue']}")
            print(f"   Action: {rec['action']}\n")

        # Provide suggested values
        print("-"*70)
        print("SUGGESTED PID VALUES:")
        print("-"*70)

        new_kp, new_ki, new_kd = 3.0, 0.5, 0.0

        # Apply recommendations
        if max_ss_error > 5:
            new_ki = 0.75
        if avg_overshoot > 10:
            new_kp = 2.4
        elif avg_overshoot > 5:
            new_kd = 0.3
        if avg_oscillations > 3:
            new_kp = 2.1
            new_ki = 0.4
        if slow_steps > len(all_metrics) / 2:
            new_kp = 3.9

        print(f"\n  Kp = {new_kp:.1f}")
        print(f"  Ki = {new_ki:.1f}")
        print(f"  Kd = {new_kd:.1f}")

        print("\nTo apply, update src/reflow_controller.rs line 48:")
        print(f"  pid_controller: PidController::new({new_kp:.1f}, {new_ki:.1f}, {new_kd:.1f}),")

    print("\n" + "="*70)
    print("\nData Summary:")
    print(f"  Total runtime: {data[-1]['time_s']:.1f}s")
    print(f"  Steps completed: {len(all_metrics)}")
    print(f"  Average overshoot: {avg_overshoot:.1f}°C")
    print(f"  Average oscillations: {avg_oscillations:.1f}")
    print(f"  Max steady-state error: {max_ss_error:.1f}°C")
    print()

if __name__ == "__main__":
    try:
        main()
    except FileNotFoundError:
        print("Error: reflow_data.csv not found!")
        print("Please run the simulation first to generate data.")
    except Exception as e:
        print(f"Error: {e}")
        import traceback
        traceback.print_exc()