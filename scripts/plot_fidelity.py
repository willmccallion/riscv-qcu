import matplotlib.pyplot as plt
import re
import sys
import os

lines = sys.stdin.readlines()
data = {} # {qubit_id: [ (time, z_value) ]}

# Regex for Hamiltonian logs: ... | X: 123 | Z: 16000
log_pattern = re.compile(r'gen_qubits\[(\d+)\].*Time:\s*(\d+)\s*\|\s*X:\s*(-?\d+)\s*\|\s*Z:\s*(-?\d+)')

found_logs = False

for line in lines:
    print(line, end='') 
    match = log_pattern.search(line)
    if match:
        found_logs = True
        q_id = int(match.group(1))
        time = int(match.group(2))
        z_val = int(match.group(4)) # We plot Z to show stability
        
        if q_id not in data: data[q_id] = []
        data[q_id].append((time, z_val))

if not found_logs:
    print("\n[ERROR] No matching logs found!")
    sys.exit(1)

plt.figure(figsize=(12, 8))

for q_id, points in data.items():
    times, z_vals = zip(*points)
    plt.plot(times, z_vals, marker='.', markersize=1, linewidth=1, label=f'Qubit {q_id}')

# 1.0 in Fixed Point 2.14 is 16384
plt.axhline(y=16384, color='g', linestyle='--', label='Ideal |0> (+1.0)')
plt.axhline(y=0, color='r', linestyle='--', label='Collapse Threshold (0.0)')
plt.axhline(y=-16384, color='k', linestyle=':', label='State |1> (-1.0)')

plt.xlabel('Simulation Cycles')
plt.ylabel('Z-Component (Fixed Point 2.14)')
plt.title('Pulse-Level Hamiltonian Control (Rabi Oscillations)')
plt.legend(loc='upper right', bbox_to_anchor=(1.15, 1))
plt.grid(True, alpha=0.3)
plt.tight_layout()

output_path = os.path.join(os.path.dirname(__file__), '../output/fidelity_plot.png')
os.makedirs(os.path.dirname(output_path), exist_ok=True)
plt.savefig(output_path)
print(f"\n[SUCCESS] Graph saved to {output_path}")
