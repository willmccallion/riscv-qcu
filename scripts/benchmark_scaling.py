import subprocess
import re
import sys
import os

# Distances to benchmark
DISTANCES = [3, 5, 7, 9, 11, 13, 15, 17, 19, 21]

# Duration per run
DURATION = 5 

# Files
DEM_FILE = "output/scaling.dem"
B8_FILE = "output/scaling.b8"

def run_command(cmd):
    """Runs a shell command and returns stdout."""
    result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
    if result.returncode != 0:
        print(f"Error running command: {cmd}")
        print(result.stderr)
        sys.exit(1)
    return result.stdout.strip()

def parse_results(output):
    """Extracts Average Latency and Throughput from Rust output."""
    latency = 0.0
    throughput = 0.0

    # Regex to find "Avg:   7.63 us" OR "Avg:   130.00 ns"
    lat_match = re.search(r"Avg:\s+([\d\.]+)\s+(us|ns)", output)
    if lat_match:
        val = float(lat_match.group(1))
        unit = lat_match.group(2)
        if unit == "ns":
            latency = val / 1000.0
        else:
            latency = val

    # Regex to find throughput in the "T= Xs" lines. 
    tput_matches = re.findall(r"\(\s*(\d+)/s\)", output)
    if tput_matches:
        tputs = [int(x) for x in tput_matches]
        if tputs:
            throughput = sum(tputs) / len(tputs)

    return latency, throughput

def main():
    # Ensure output dir exists
    if not os.path.exists("output"):
        os.makedirs("output")

    print(f"{'Dist':<5} | {'Detectors':<10} | {'Latency (us)':<12} | {'Throughput (Hz)':<15} | {'Status'}")
    print("-" * 65)

    for d in DISTANCES:
        # Generate Data using the RUST generator
        gen_cmd = (
            f"cargo run --quiet --release --bin qcu_host -- gen "
            f"--size {d} "
            f"--shots 10000 "
            f"--p 0.05 " 
            f"--dem {DEM_FILE} "
            f"--b8 {B8_FILE}"
        )
        run_command(gen_cmd)

        # Calculate number of detectors (Size * Size)
        num_detectors = d * d

        # Run Stream Benchmark
        bench_cmd = (
            f"cargo run --quiet --release --bin qcu_host -- stream "
            f"--dem {DEM_FILE} "
            f"--b8 {B8_FILE} "
            f"--freq 100000 " 
            f"--duration {DURATION} "
            f"--detectors {num_detectors}"
        )

        try:
            bench_output = run_command(bench_cmd)
            lat, tput = parse_results(bench_output)

            # Threshold: 10us is a reasonable soft-real-time limit
            status = "[ OK ]" if lat < 10.0 else "[ SLOW ]"

            print(f"{d:<5} | {num_detectors:<10} | {lat:<12.2f} | {tput:<15.0f} | {status}")
        except KeyboardInterrupt:
            print("\nBenchmark interrupted by user.")
            sys.exit(0)

    print("-" * 65)

    # Cleanup
    if os.path.exists(DEM_FILE): os.remove(DEM_FILE)
    if os.path.exists(B8_FILE): os.remove(B8_FILE)

if __name__ == "__main__":
    main()
