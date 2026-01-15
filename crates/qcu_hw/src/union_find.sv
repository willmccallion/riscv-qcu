module union_find #(
    parameter WIDTH = 32
)(
    input  logic             clk,
    input  logic             rst_n,

    // Control Interface
    input  logic             start,
    input  logic [WIDTH-1:0] node_in,
    output logic [WIDTH-1:0] root_out,
    output logic             done,
    output logic             busy,

    // Memory Interface
    output logic             mem_rd_en,
    output logic [WIDTH-1:0] mem_addr,
    input  logic [WIDTH-1:0] mem_rdata,
    input  logic             mem_ready
);

    typedef enum logic [2:0] {
        IDLE      = 3'd0,
        READ_REQ  = 3'd1,
        READ_WAIT = 3'd2,
        CHECK     = 3'd3,
        DONE_ST   = 3'd4
    } state_t;

    state_t state, next_state;
    logic [WIDTH-1:0] curr_node, next_node;
    logic [WIDTH-1:0] rdata_reg;
    logic             rdata_valid;

    // Sequential Logic
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

    // Combinational Logic
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
