//! Hardware-in-the-loop interface for real-time quantum hardware simulation.
//!
//! Provides a TCP-based communication interface to a Verilator simulation
//! of quantum hardware. Enables real-time monitoring and control of quantum
//! qubit states, error detection, and correction operations. Used for
//! demonstrating closed-loop error correction on simulated quantum systems.

use anyhow::Result;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

/// Command opcodes for hardware bridge protocol.
///
/// Defines the binary protocol for communicating with the Verilator simulation.
/// Commands are sent as single-byte opcodes followed by command-specific data.
const CMD_STEP: u8 = 0x01;

/// Command opcode for writing to a memory-mapped register.
///
/// Sent as the first byte of a write transaction, followed by the 32-bit
/// address and 32-bit data value. The simulation acknowledges the write
/// with a 32-bit response.
const CMD_WRITE: u8 = 0x02;

/// Command opcode for reading from a memory-mapped register.
///
/// Sent as the first byte of a read transaction, followed by the 32-bit
/// address. The simulation responds with the 32-bit register value.
const CMD_READ: u8 = 0x03;

/// Memory-mapped register addresses in the hardware simulation.
///
/// Defines the register layout for controlling and reading quantum hardware
/// state. These addresses correspond to MMIO registers in the Verilator model.
const ADDR_ENABLE: u32 = 0x4000_0000;

/// Register address for triggering correction pulses on qubits.
///
/// Writing a bitmask to this register applies correction pulses to the
/// qubits corresponding to set bits. The pulse duration and strength are
/// controlled by other registers. Writing zero clears all active pulses.
const ADDR_PULSE: u32 = 0x4000_0001;

/// Register address for reading qubit error syndrome measurements.
///
/// Reading from this register returns a bitmask indicating which qubits
/// have detected errors. Each bit corresponds to one qubit in the grid,
/// with bit 0 representing qubit 0, bit 1 representing qubit 1, etc.
const ADDR_MEASURE: u32 = 0x4000_0002;

/// Register address for configuring Rabi frequency (pulse strength).
///
/// Writing a value to this register sets the strength of correction pulses
/// applied to qubits. Higher values correspond to stronger pulses and faster
/// rotations. The value 468 corresponds to a specific Rabi frequency in
/// the physics simulation's units.
const ADDR_RABI: u32 = 0x4000_0003;

/// TCP bridge for communicating with Verilator hardware simulation.
///
/// Maintains a persistent TCP connection to the simulation and provides
/// methods for stepping the simulation, reading quantum state, and applying
/// correction pulses. All operations are synchronous and block until the
/// simulation responds.
pub struct HardwareBridge {
    stream: TcpStream,
}

impl HardwareBridge {
    /// Establishes a TCP connection to the Verilator simulation server.
    ///
    /// Connects to the specified address and port, enabling TCP_NODELAY to
    /// reduce latency for real-time control. The connection remains open
    /// for the lifetime of the HardwareBridge instance.
    ///
    /// # Arguments
    ///
    /// * `addr` - Server address in "host:port" format (e.g., "127.0.0.1:8000")
    ///
    /// # Returns
    ///
    /// Ok(HardwareBridge) on successful connection, or an error if the
    /// connection cannot be established.
    pub fn connect(addr: &str) -> Result<Self> {
        let stream = TcpStream::connect(addr)?;
        stream.set_nodelay(true)?;
        Ok(Self { stream })
    }

    /// Advances the hardware simulation by the specified number of clock cycles.
    ///
    /// Sends a STEP command to the simulation server with the cycle count,
    /// then waits for acknowledgment. This allows fine-grained control over
    /// simulation timing for precise error detection and correction timing.
    ///
    /// # Arguments
    ///
    /// * `cycles` - Number of clock cycles to advance the simulation
    ///
    /// # Returns
    ///
    /// Ok(()) on success, or an error if the command fails or connection is lost.
    pub fn step(&mut self, cycles: u32) -> Result<()> {
        self.stream.write_all(&[CMD_STEP])?;
        self.stream.write_all(&cycles.to_le_bytes())?;
        let mut ack = [0u8; 4];
        self.stream.read_exact(&mut ack)?;
        Ok(())
    }

    /// Writes a 32-bit value to a memory-mapped register in the simulation.
    ///
    /// Sends a WRITE command with the address and data, then waits for
    /// acknowledgment. Used to configure hardware peripherals, trigger
    /// operations, and apply correction pulses to qubits.
    ///
    /// # Arguments
    ///
    /// * `addr` - 32-bit memory-mapped address to write
    /// * `data` - 32-bit value to write
    ///
    /// # Returns
    ///
    /// Ok(()) on success, or an error if the write fails or connection is lost.
    pub fn write(&mut self, addr: u32, data: u32) -> Result<()> {
        self.stream.write_all(&[CMD_WRITE])?;
        self.stream.write_all(&addr.to_le_bytes())?;
        self.stream.write_all(&data.to_le_bytes())?;
        let mut ack = [0u8; 4];
        self.stream.read_exact(&mut ack)?;
        Ok(())
    }

    /// Reads a 32-bit value from a memory-mapped register in the simulation.
    ///
    /// Sends a READ command with the address, then receives and returns the
    /// register value. Used to read qubit error states, status registers, and
    /// measurement results from the quantum hardware simulation.
    ///
    /// # Arguments
    ///
    /// * `addr` - 32-bit memory-mapped address to read
    ///
    /// # Returns
    ///
    /// Ok(value) on success with the 32-bit register value, or an error if
    /// the read fails or connection is lost.
    pub fn read(&mut self, addr: u32) -> Result<u32> {
        self.stream.write_all(&[CMD_READ])?;
        self.stream.write_all(&addr.to_le_bytes())?;
        let mut data = [0u8; 4];
        self.stream.read_exact(&mut data)?;
        Ok(u32::from_le_bytes(data))
    }
}

/// Runs the hardware-in-the-loop demonstration.
///
/// Connects to the Verilator simulation, initializes the physics engine,
/// and enters a control loop that: (1) steps the hardware simulation,
/// (2) reads qubit error states, (3) applies correction pulses when errors
/// are detected, (4) updates the event history log, and (5) renders a
/// real-time dashboard showing qubit states. The loop runs at approximately
/// 30 FPS for responsive visualization. Detection interval is 25 cycles for
/// fast error detection, pulse strength is 600 (Rabi frequency), and pulse
/// duration is 110 cycles to ensure full 180-degree rotations (π radians)
/// for error corrections.
///
/// # Returns
///
/// Ok(()) on success, or an error if connection or I/O operations fail.
pub fn run_hil_demo() -> Result<()> {
    // ANSI escape code for green text color.
    //
    // Used to display stable qubit states (no errors detected) in the
    // real-time dashboard. Green indicates coherent quantum states.
    const GREEN: &str = "\x1b[32m";

    // ANSI escape code for red text color.
    //
    // Used to display error states (qubits with detected errors) in the
    // real-time dashboard. Red indicates qubits that require correction.
    const RED: &str = "\x1b[31m";

    // ANSI escape code for yellow text color.
    //
    // Used to display correction operations in progress in the event log.
    // Yellow indicates that correction pulses are being applied.
    const YELLOW: &str = "\x1b[33m";

    // ANSI escape code to reset text formatting.
    //
    // Restores default terminal colors and formatting after applying color
    // codes. Must be used after each colored text segment to prevent color
    // bleeding into subsequent output.
    const RESET: &str = "\x1b[0m";

    // ANSI escape code to clear the terminal screen and move cursor to home.
    //
    // Clears all terminal content and positions the cursor at the top-left
    // corner. Used to refresh the dashboard display on each update cycle,
    // creating a real-time updating visualization.
    const CLEAR: &str = "\x1b[2J\x1b[1;1H";

    println!("Connecting to Quantum Hardware (Verilator)...");
    let mut hw = HardwareBridge::connect("127.0.0.1:8000")?;

    hw.write(ADDR_ENABLE, 1)?;
    hw.write(ADDR_RABI, 468)?;

    let mut total_cycles = 0;
    let mut history: Vec<String> = Vec::new();

    loop {
        hw.step(25)?;
        total_cycles += 25;

        let syndrome = hw.read(ADDR_MEASURE)?;

        let correction_str = if syndrome != 0 {
            hw.write(ADDR_PULSE, syndrome)?;
            hw.step(110)?;
            hw.write(ADDR_PULSE, 0)?;
            format!("{}CORRECTING{}", YELLOW, RESET)
        } else {
            format!("{}STABLE    {}", GREEN, RESET)
        };

        if syndrome != 0 || total_cycles % 5000 == 0 {
            let log_entry = format!(
                "Cycle {:8} | Errors: {:09b} | Status: {}",
                total_cycles, syndrome, correction_str
            );
            history.push(log_entry);
            if history.len() > 10 {
                history.remove(0);
            }
        }

        print!("{}", CLEAR);
        println!("========================================");
        println!("   QUANTUM CONTROL UNIT - LIVE STATUS   ");
        println!("========================================");
        println!("Total Cycles: {}", total_cycles);
        println!("----------------------------------------");
        println!("Physical Qubit Grid (3x3):");
        println!();

        for row in 0..3 {
            print!("   ");
            for col in 0..3 {
                let idx = row * 3 + col;
                let is_error = (syndrome >> idx) & 1 == 1;

                if is_error {
                    print!("{}[ X ]{} ", RED, RESET);
                } else {
                    print!("{}[ O ]{} ", GREEN, RESET);
                }
            }
            println!("\n");
        }
        println!("   [O] = Coherent  [X] = Error/Decay");
        println!("----------------------------------------");
        println!("Event Log:");
        for entry in &history {
            println!("   {}", entry);
        }
        println!("========================================");

        thread::sleep(Duration::from_millis(30));
    }
}
