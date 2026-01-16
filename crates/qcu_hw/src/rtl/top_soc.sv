/**
 * @file top_soc.sv
 * @brief Top-level System-on-Chip module integrating quantum hardware peripherals.
 *
 * Implements the system bus interface and address decoding logic for memory-mapped
 * quantum hardware peripherals. The module routes bus transactions to the appropriate
 * peripheral based on the upper 16 bits of the address. Currently integrates the
 * qubit grid physics engine at address 0x4000_0000. The bus protocol supports
 * both read and write transactions with chip select and write enable signals
 * for transaction qualification.
 */
module top_soc (
    input  logic        clk,       /**< System clock */
    input  logic        rst_n,     /**< Active-low asynchronous reset */

    input  logic        bus_cs,    /**< Bus chip select (transaction valid) */
    input  logic        bus_we,    /**< Bus write enable (1=write, 0=read) */
    input  logic [31:0] bus_addr,   /**< 32-bit memory-mapped address */
    input  logic [31:0] bus_wdata,  /**< 32-bit write data */
    output logic [31:0] bus_rdata   /**< 32-bit read data */
);

    /**
     * Address decode signal for physics engine peripheral.
     *
     * Asserted when bus_cs is active and the upper 16 address bits match
     * 0x4000, selecting the qubit grid physics engine. The lower 16 bits
     * are passed to the peripheral for register selection.
     */
    logic physics_sel;
    assign physics_sel = bus_cs && (bus_addr[31:16] == 16'h4000);

    /**
     * Debug logging for physics engine bus transactions.
     *
     * Prints a message whenever a write transaction targets the physics
     * engine, showing the address and data for debugging purposes.
     */
    always_ff @(posedge clk) begin
        if (physics_sel && bus_we) begin
            $display("[HW-TOP] Write to Physics! Addr: %h Data: %h", bus_addr, bus_wdata);
        end
    end

    qubit_grid u_physics (
        .clk(clk),
        .rst_n(rst_n),
        .cs(physics_sel),
        .we(bus_we),
        .addr(bus_addr[3:0]), 
        .wdata(bus_wdata),
        .rdata(bus_rdata)
    );

endmodule
