/**
 * @file hamiltonian_engine.sv
 * @brief Quantum qubit state evolution engine using symplectic Euler integration.
 *
 * Implements a single qubit's quantum state evolution under Hamiltonian dynamics
 * with noise and control pulses. The qubit state is represented as a 2D vector
 * (X, Z) in the Bloch sphere, evolved using symplectic Euler integration to
 * preserve energy better than standard Euler methods. The engine models T1
 * relaxation by resetting the qubit to |0⟩ when the state vector magnitude
 * becomes too small. Noise is injected as random phase rotations, and control
 * pulses can be applied to perform quantum gates or error corrections.
 *
 * @param SEED Initial seed for the Xorshift32 pseudo-random number generator
 */
module hamiltonian_engine #(
    parameter SEED = 32'h1234_5678
)(
    input  logic        clk,           /**< System clock */
    input  logic        rst_n,          /**< Active-low asynchronous reset */
    input  logic        enable,         /**< Enable state evolution (clock gating) */
    input  logic        apply_pulse,    /**< Apply control pulse (correction gate) */
    input  logic [15:0] pulse_strength, /**< Pulse rotation angle (Rabi frequency) */
    output logic        measurement     /**< Measurement result (Z < 0 indicates |1⟩) */
);

    /**
     * Fixed-point number format: 2.14 (2 integer bits, 14 fractional bits).
     *
     * The value 1.0 is represented as 16384 (0x4000). This format provides
     * sufficient precision for quantum state representation while enabling
     * efficient integer arithmetic operations. Signed 16-bit values allow
     * representation of the full Bloch sphere range [-1, 1].
     */
    typedef logic signed [15:0] fixed_t;
    fixed_t state_x;        /**< X component of qubit state vector */
    fixed_t state_z;        /**< Z component of qubit state vector */
    logic [31:0] rng;       /**< Xorshift32 PRNG state for noise generation */

    /**
     * Initial state: qubit starts in |0⟩ state.
     *
     * Initializes state vector to |0⟩ (X=0, Z=1.0 in fixed-point).
     * The PRNG is seeded with the module parameter for deterministic
     * noise generation.
     */
    initial begin
        state_x = 16'h0000;
        state_z = 16'h4000;
        rng = SEED;
    end

    /**
     * Xorshift32 pseudo-random number generator.
     *
     * Generates pseudo-random values for noise injection. Uses three
     * XOR-shift operations (13, 17, 5 bit shifts) to produce a uniform
     * distribution. Resets to SEED if the state becomes zero to avoid
     * getting stuck.
     */
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) rng <= SEED;
        else begin
            logic [31:0] x = rng;
            x = x ^ (x << 13);
            x = x ^ (x >> 17);
            x = x ^ (x << 5);
            rng <= (x == 0) ? SEED : x;
        end
    end

    fixed_t theta;
    fixed_t noise; 
    
    /**
     * Noise and control pulse angle calculation.
     *
     * Computes the rotation angle theta for state evolution. When apply_pulse
     * is asserted, uses the negative pulse strength for correction rotations.
     * Otherwise, injects random noise with magnitude approximately +/- 0.002
     * radians derived from the PRNG state.
     */
    always_comb begin
          noise = { {13{rng[2]}}, rng[2:0] };
        
        if (apply_pulse) theta = -pulse_strength; 
        else theta = noise;
    end

    /**
     * Symplectic Euler integration for quantum state evolution.
     *
     * Implements symplectic Euler method which preserves energy (orbit radius)
     * much better than standard Euler integration. The algorithm: (1) calculate
     * new X using current Z, (2) calculate new Z using the NEW X (symplectic
     * property), (3) perform energy rescue (renormalization) if state vector
     * magnitude becomes too small, simulating T1 relaxation that resets the
     * qubit to |0⟩. Includes optimized logging that prints state every ~4000
     * cycles to reduce simulation overhead.
     */
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            state_x <= 16'h0000;
            state_z <= 16'h4000;
        end else if (enable) begin
            logic signed [31:0] mult_x = state_z * theta;
            fixed_t delta_x = 16'((mult_x + 32'h2000) >>> 14);
            fixed_t next_x = state_x - delta_x;
            
            logic signed [31:0] mult_z = next_x * theta;
            fixed_t delta_z = 16'((mult_z + 32'h2000) >>> 14);
            fixed_t next_z = state_z + delta_z;
            
            if ((next_z > -1000 && next_z < 1000) && (next_x > -1000 && next_x < 1000)) begin
                state_x <= 0;
                state_z <= 16'h4000;
            end else begin
                state_x <= next_x;
                state_z <= next_z;
            end
            
            if ((rng & 32'hFFF) == 0) begin
                $display("[HW-PHYS] %m | Time: %0t | X: %d | Z: %d", $time, next_x, next_z);
            end
        end
    end

    assign measurement = (state_z < 0);

endmodule
