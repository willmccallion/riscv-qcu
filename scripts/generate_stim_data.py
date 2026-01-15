import stim
import pathlib
import argparse
import sys

def generate_data(distance, shots, noise, dem_path, b8_path):
    print(f"--> Generating Surface Code (d={distance}, p={noise}) using Stim...")

    # Create a standard surface code circuit
    circuit = stim.Circuit.generated(
        "surface_code:rotated_memory_z",
        rounds=distance, # Real circuits have rounds ~= distance
        distance=distance,
        after_clifford_depolarization=noise,
        after_reset_flip_probability=noise,
        before_measure_flip_probability=noise,
        before_round_data_depolarization=noise
    )

    # Generate DEM and FLATTEN it (remove repeats so simple parsers work)
    dem = circuit.detector_error_model(decompose_errors=True).flattened()

    pathlib.Path(dem_path).parent.mkdir(parents=True, exist_ok=True)
    with open(dem_path, "w") as f:
        f.write(str(dem))

    # Sample shots
    sampler = circuit.compile_detector_sampler()
    sampler.sample_write(
        shots=shots,
        filepath=str(b8_path),
        format="b8"
    )

    print(f"    Detectors: {circuit.num_detectors}")
    print(f"    Graph and Shots written to {pathlib.Path(dem_path).parent}")

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--distance", type=int, required=True)
    parser.add_argument("--shots", type=int, default=1000)
    parser.add_argument("--noise", type=float, default=0.005)
    parser.add_argument("--out_dem", type=str, default="output/bench.dem")
    parser.add_argument("--out_b8", type=str, default="output/bench.b8")
    args = parser.parse_args()

    generate_data(args.distance, args.shots, args.noise, args.out_dem, args.out_b8)
