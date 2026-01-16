/**
 * @file noise_channel.sv
 * @brief Stochastic error generation channel with accumulation threshold.
 *
 * Implements a noise model that generates random error events based on a
 * linear feedback shift register (LFSR) pseudo-random number generator.
 * Errors accumulate over time until a threshold is reached, at which point
 * the error_state signal is asserted. The accumulation mechanism models
 * gradual decoherence processes. Corrections can be applied to reset the
 * accumulation counter and clear the error state, simulating active error
 * correction operations.
 *
 * @param WIDTH Bit width for accumulation counter (default 32)
 * @param THRESHOLD Accumulation value that triggers error state (default 31)
 * @param SEED Initial LFSR state for pseudo-random number generation
 */
module noise_channel #(
    parameter WIDTH = 32,
    parameter THRESHOLD = 32'h0000_001F,
    parameter SEED = 32'hDEAD_BEEF
)(
    input  logic             clk,              /**< System clock */
    input  logic             rst_n,             /**< Active-low asynchronous reset */
    input  logic             enable_evolution,  /**< Enable error accumulation */
    input  logic             apply_correction,  /**< Reset accumulation and clear error */
    output logic             error_state       /**< Asserted when threshold exceeded */
);

    logic [31:0] lfsr;         /**< Linear feedback shift register for PRNG */
    logic [31:0] accumulation;  /**< Error accumulation counter */

    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            lfsr <= SEED;
        end else begin
            if (lfsr == 32'b0) lfsr <= SEED ^ 32'h1; 
            else lfsr <= {lfsr[30:0], lfsr[31] ^ lfsr[21] ^ lfsr[1] ^ lfsr[0]};
        end
    end

    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            accumulation <= '0;
            error_state  <= 1'b0;
        end else begin
            if (apply_correction) begin
                accumulation <= '0;
                error_state  <= 1'b0;
            end else if (enable_evolution && !error_state) begin
                /**
                 * Error accumulation with reduced probability.
                 *
                 * Checks 3 LFSR bits (1/8 chance) to slow down error rate
                 * compared to single-bit checks. This models gradual decoherence
                 * processes rather than instantaneous errors.
                 */
                if (lfsr[2:0] == 3'b111) begin
                    accumulation <= accumulation + 32'd1;
                end
                
                if (accumulation > THRESHOLD) begin
                    error_state <= 1'b1;
                    $display("[HW-NOISE] %m | ERROR TRIGGERED! Acc: %d", accumulation);
                end
            end
        end
    end
endmodule
