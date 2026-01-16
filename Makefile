.PHONY: all kernel stream test clean gen hil

# Default target
all: kernel

# Build and Boot Firmware (QEMU)
kernel:
	@./scripts/run.py kernel

# Run Host Benchmark (Stream)
stream:
	@./scripts/run.py stream --freq 40000

# Run Hardware-in-the-Loop Demo (TCP Bridge)
hil:
	@./scripts/run.py hil

# Generate fresh data
gen:
	@./scripts/run.py gen --size 5 --shots 10000

# Clean artifacts
clean:
	@cargo clean
	@rm -rf output
