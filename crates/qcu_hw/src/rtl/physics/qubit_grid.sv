/**
 * @file qubit_grid.sv
 * @brief 3x3 grid of quantum qubits with individual Hamiltonian engines.
 *
 * Implements a grid of 9 qubits, each with its own Hamiltonian evolution engine.
 * Provides a memory-mapped interface for enabling/disabling physics simulation,
 * applying correction pulses to individual qubits, and reading error states.
 * The module acts as a register file that controls the physics engines and
 * aggregates their measurement outputs into a single syndrome word. Used for
 * hardware-in-the-loop testing of quantum error correction algorithms.
 */
module qubit_grid (
    input  logic        clk,     /**< System clock */
    input  logic        rst_n,   /**< Active-low asynchronous reset */
    
    input  logic        cs,      /**< Chip select (register access valid) */
    input  logic        we,      /**< Write enable (1=write, 0=read) */
    input  logic [3:0]  addr,    /**< 4-bit register address */
    input  logic [31:0] wdata,   /**< 32-bit write data */
    output logic [31:0] rdata    /**< 32-bit read data */
);

    logic [8:0] pulse_active;      /**< Per-qubit pulse enable signals (one-hot) */
    logic [8:0] errors;              /**< Per-qubit measurement results (syndrome bits) */
    logic physics_running;          /**< Global enable for all physics engines */
    logic [15:0] pulse_strength_reg; /**< Shared pulse strength (Rabi frequency) for all qubits */ 

    genvar i;
    generate
        for (i = 0; i < 9; i++) begin : gen_qubits
            hamiltonian_engine #(
                .SEED(32'hDEAD_BEEF ^ (i * 32'h9E3779B9)) 
            ) u_physics_core (
                .clk(clk),
                .rst_n(rst_n),
                .enable(physics_running),
                .apply_pulse(pulse_active[i]),
                .pulse_strength(pulse_strength_reg),
                .measurement(errors[i])
            );
        end
    endgenerate

    /**
     * Register write logic for qubit grid control.
     *
     * Handles writes to memory-mapped registers: enable/disable physics
     * simulation, trigger correction pulses on individual qubits, and set
     * pulse strength (Rabi frequency). Pulses are momentary triggers in
     * this bus protocol, so the controller must sustain writes for long
     * pulse durations. Default pulse strength is 500 (Rabi frequency units).
     */
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            physics_running <= 1'b0;
            pulse_active    <= '0;
            pulse_strength_reg <= 16'd500;
        end else begin
            pulse_active <= '0; 
            
            if (cs && we) begin
                case (addr)
                    4'h0: physics_running <= wdata[0];
                    4'h1: pulse_active    <= wdata[8:0];
                    4'h3: pulse_strength_reg <= wdata[15:0];
                    default: ;
                endcase
            end
        end
    end

    always_comb begin
        rdata = '0;
        if (cs && !we) begin
            case (addr)
                4'h0: rdata = {31'b0, physics_running};
                4'h2: rdata = {23'b0, errors};
                4'h3: rdata = {16'b0, pulse_strength_reg};
                default: rdata = '0;
            endcase
        end
    end
endmodule
