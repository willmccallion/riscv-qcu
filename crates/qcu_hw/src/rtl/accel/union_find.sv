/**
 * @file union_find.sv
 * @brief Hardware-accelerated union-find path compression engine.
 *
 * Implements a state machine that performs path compression for the union-find
 * data structure used in quantum error correction decoding. The module traverses
 * the parent pointer chain from a given node to its root, reading parent values
 * from external memory. The path compression optimization is performed in
 * hardware to reduce latency compared to software implementations. The module
 * uses a memory interface to access the parent array stored in system memory,
 * enabling integration with the RISC-V processor's memory subsystem.
 *
 * @param WIDTH Bit width of node indices and memory data (default 32)
 */
module union_find #(
    parameter WIDTH = 32
)(
    input  logic             clk,      /**< System clock */
    input  logic             rst_n,     /**< Active-low asynchronous reset */

    input  logic             start,     /**< Assert to begin find operation */
    input  logic [WIDTH-1:0] node_in,   /**< Starting node index for find */
    output logic [WIDTH-1:0] root_out,  /**< Root node index result */
    output logic             done,      /**< Asserted when find completes */
    output logic             busy,      /**< Asserted during active operation */

    output logic             mem_rd_en, /**< Memory read request enable */
    output logic [WIDTH-1:0] mem_addr,   /**< Memory address (parent array index) */
    input  logic [WIDTH-1:0] mem_rdata,  /**< Memory read data (parent value) */
    input  logic             mem_ready   /**< Memory transaction complete */
);

    /**
     * State machine enumeration for find operation control flow.
     *
     * The state machine progresses through: IDLE (waiting for start), READ_REQ
     * (initiate memory read), READ_WAIT (wait for memory response), CHECK
     * (compare parent with current node), and DONE_ST (operation complete).
     * The CHECK state implements the path compression loop, continuing until
     * a self-referential parent is found (indicating the root).
     */
    typedef enum logic [2:0] {
        IDLE      = 3'd0, /**< Idle state, waiting for start signal */
        READ_REQ  = 3'd1, /**< Issue memory read request for parent value */
        READ_WAIT = 3'd2, /**< Wait for memory read data to become valid */
        CHECK     = 3'd3, /**< Compare parent with current node (path compression) */
        DONE_ST   = 3'd4  /**< Find complete, root identified */
    } state_t;

    state_t state, next_state;              /**< Current and next state registers */
    logic [WIDTH-1:0] curr_node, next_node; /**< Current node index in traversal */
    logic [WIDTH-1:0] rdata_reg;             /**< Registered memory read data */
    logic             rdata_valid;           /**< Flag indicating read data is valid */

    /**
     * Sequential state machine and data path logic.
     *
     * Updates state, current node index, and registered memory data on
     * each clock edge. The rdata_valid flag is set when memory data arrives
     * and cleared when entering the CHECK state.
     */
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            state       <= IDLE;
            curr_node   <= '0;
            rdata_reg   <= '0;
            rdata_valid <= 1'b0;
        end else begin
            state     <= next_state;
            curr_node <= next_node;

            if (mem_ready) begin
                rdata_reg   <= mem_rdata;
                rdata_valid <= 1'b1;
            end 
            else if (state == CHECK) begin
                rdata_valid <= 1'b0;
            end
        end
    end

    /**
     * Combinational next-state and output logic.
     *
     * Computes the next state based on current state and inputs, drives
     * memory interface signals, and generates done and busy outputs.
     * Implements the path compression loop that continues until a
     * self-referential parent is found.
     */
    always_comb begin
        next_state = state;
        next_node  = curr_node;

        busy       = 1'b1;
        done       = 1'b0;
        root_out   = curr_node;
        mem_rd_en  = 1'b0;
        mem_addr   = curr_node;

        case (state)
            IDLE: begin
                busy = 1'b0;
                if (start) begin
                    next_node  = node_in;
                    next_state = READ_REQ;
                end
            end

            READ_REQ: begin
                mem_rd_en = 1'b1;
                mem_addr  = curr_node;
                next_state = READ_WAIT;
            end

            READ_WAIT: begin
                if (rdata_valid || mem_ready) begin
                    next_state = CHECK;
                end
            end

            CHECK: begin
                if (rdata_reg == curr_node) begin
                    next_state = DONE_ST;
                end else begin
                    next_node  = rdata_reg;
                    next_state = READ_REQ;
                end
            end

            DONE_ST: begin
                busy = 1'b0;
                done = 1'b1;

                if (start) begin
                    next_node  = node_in;
                    next_state = READ_REQ;
                end else begin
                    next_state = IDLE;
                end
            end

            default: next_state = IDLE;
        endcase
    end

endmodule
