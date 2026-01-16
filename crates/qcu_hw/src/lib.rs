//! Hardware simulation interface for union-find accelerator testing.
//!
//! Provides a Rust interface to hardware-accelerated union-find operations
//! implemented in Verilog/SystemVerilog. The interface communicates with
//! a Verilator simulation via FFI bindings to test hardware acceleration
//! of the decoder's critical path operations.

/// Foreign function interface to hardware simulation functions.
///
/// These functions are implemented in C++ and linked with the Verilator
/// simulation. They provide low-level control over the hardware accelerator's
/// state machine and data paths. All functions are marked unsafe because
/// they interact with external C++ code that may have different safety
/// guarantees than Rust.
unsafe extern "C" {
    /// Initializes the hardware accelerator with parent array data.
    ///
    /// Loads the parent array into the accelerator's memory, preparing it
    /// for union-find operations. Must be called before any find_root calls.
    ///
    /// # Arguments
    ///
    /// * `data` - Pointer to the parent array (u32 values)
    /// * `len` - Length of the parent array
    fn hw_init(data: *const u32, len: usize);

    /// Shuts down the hardware accelerator.
    ///
    /// Cleans up hardware state and releases resources. Should be called
    /// when the accelerator is no longer needed.
    fn hw_shutdown();

    /// Advances the hardware simulation by one clock cycle.
    ///
    /// Steps the Verilator simulation forward, allowing the accelerator's
    /// state machine to progress. Must be called repeatedly until operations
    /// complete.
    fn hw_step();

    /// Sets input values for the accelerator.
    ///
    /// Configures the start flag and node index for the next find operation.
    /// The start flag indicates whether to begin a new operation.
    ///
    /// # Arguments
    ///
    /// * `start` - 1 to start operation, 0 to continue
    /// * `node` - Node index for the find operation
    fn hw_set_input(start: i32, node: i32);

    /// Reads the root node result from the accelerator.
    ///
    /// Returns the root node index found by the hardware find operation.
    /// Only valid after hw_is_done() returns non-zero.
    ///
    /// # Returns
    ///
    /// The root node index (as i32, cast to u32 by caller)
    fn hw_get_root() -> i32;

    /// Checks whether the current operation is complete.
    ///
    /// Returns non-zero when the hardware has finished processing the
    /// current find operation and results are ready.
    ///
    /// # Returns
    ///
    /// Non-zero if done, zero if still processing
    fn hw_is_done() -> i32;
}

/// Wrapper for hardware-accelerated union-find operations.
///
/// Provides a safe Rust interface to the hardware accelerator, managing
/// initialization and cleanup automatically. The accelerator performs
/// path compression in hardware to reduce latency compared to software
/// implementations.
pub struct UnionFindAccel {
    /// Phantom data marker to prevent construction without initialization.
    _marker: std::marker::PhantomData<()>,
}

impl UnionFindAccel {
    /// Initializes the hardware accelerator with parent array data.
    ///
    /// Loads the parent array into the accelerator's memory and prepares
    /// it for find operations. The parent array must remain valid for the
    /// lifetime of the UnionFindAccel instance.
    ///
    /// # Arguments
    ///
    /// * `parent_array` - Array of parent pointers for the union-find structure
    ///
    /// # Returns
    ///
    /// A new UnionFindAccel instance ready for find operations.
    pub fn new(parent_array: &[u32]) -> Self {
        unsafe {
            hw_init(parent_array.as_ptr(), parent_array.len());
        }
        Self {
            _marker: std::marker::PhantomData,
        }
    }

    /// Performs a hardware-accelerated find operation.
    ///
    /// Finds the root of the set containing the specified node using the
    /// hardware accelerator. The operation involves setting inputs, stepping
    /// the simulation until completion, and reading the result. Includes
    /// a timeout check to detect hardware hangs.
    ///
    /// # Arguments
    ///
    /// * `node_idx` - Node index to find the root for
    ///
    /// # Returns
    ///
    /// The root node index of the set containing node_idx.
    ///
    /// # Panics
    ///
    /// Panics if the hardware accelerator does not complete within 2000
    /// simulation cycles, indicating a potential hardware bug or timeout.
    pub fn find_root(&self, node_idx: u32) -> u32 {
        unsafe {
            hw_set_input(1, node_idx as i32);
            hw_step();

            hw_set_input(0, node_idx as i32);

            let mut cycles = 0;
            while hw_is_done() == 0 {
                hw_step();
                cycles += 1;
                if cycles > 2000 {
                    panic!(
                        "Hardware Accelerator Timeout on node {}! Cycles: {}",
                        node_idx, cycles
                    );
                }
            }
            hw_get_root() as u32
        }
    }
}

impl Drop for UnionFindAccel {
    /// Shuts down the hardware accelerator on drop.
    ///
    /// Ensures proper cleanup of hardware resources when the accelerator
    /// instance is dropped, preventing resource leaks in the simulation.
    fn drop(&mut self) {
        unsafe {
            hw_shutdown();
        }
    }
}
