#include "Vunion_find.h"
#include "verilated.h"
#include <cstdio>

// Global Simulation State
static Vunion_find *top = nullptr;
static vluint64_t main_time = 0;
static std::vector<uint32_t> ram_memory;

// Pipeline state for memory latency
static bool last_rd_en = false;
static uint32_t last_addr = 0;

extern "C" {

void hw_init(const uint32_t *data, size_t len) {
  if (top)
    delete top;
  top = new Vunion_find;

  ram_memory.assign(data, data + len);
  // Ensure sufficient memory size
  if (ram_memory.size() < 1024)
    ram_memory.resize(1024, 0);

  // Reset Pipeline
  last_rd_en = false;
  last_addr = 0;

  // Reset Hardware
  top->clk = 0;
  top->rst_n = 0;
  top->start = 0;
  top->mem_ready = 0;
  top->eval();

  top->rst_n = 1;
  top->eval();

  // fprintf(stderr, "[SIM] Init Complete. RAM Size: %lu\n", ram_memory.size());
}

void hw_shutdown() {
  if (top) {
    delete top;
    top = nullptr;
  }
}

void hw_step() {
  if (!top)
    return;

  if (last_rd_en) {
    if (last_addr < ram_memory.size()) {
      top->mem_rdata = ram_memory[last_addr];
    } else {
      top->mem_rdata = 0;
    }
    top->mem_ready = 1;
  } else {
    top->mem_ready = 0;
  }

  top->clk = 1;
  top->eval();
  main_time++;

  last_rd_en = top->mem_rd_en;
  last_addr = top->mem_addr;

  top->clk = 0;
  top->eval();
  main_time++;
}

void hw_set_input(int start, int node) {
  if (top) {
    top->start = start;
    top->node_in = node;
    // fprintf(stderr, "[SIM] INPUT: Start=%d Node=%d\n", start, node);
  }
}

int hw_get_root() { return top ? top->root_out : 0; }
int hw_is_done() { return top ? top->done : 0; }
}
