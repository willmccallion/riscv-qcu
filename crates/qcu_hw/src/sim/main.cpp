/**
 * @file main.cpp
 * @brief Verilator simulation main loop with TCP server interface.
 *
 * Implements a SystemC-style simulation wrapper around the Verilator-generated
 * top-level System-on-Chip module. Provides a TCP server interface that accepts
 * commands from the Rust host controller to step the simulation, read/write
 * memory-mapped registers, and control quantum hardware peripherals. The server
 * listens on port 8000 and processes commands in a blocking loop until the
 * connection is closed or an exit command is received.
 */

#include "Vtop_soc.h"
#include "verilated.h"
#include <cstring>
#include <iostream>
#include <memory>
#include <netinet/in.h>
#include <sys/socket.h>
#include <unistd.h>
#include <vector>

/**
 * Global simulation time counter for SystemC compatibility.
 *
 * Incremented on each clock edge to provide a monotonically increasing
 * time value for simulation logging and debugging. Used by Verilator's
 * time stamp callback function.
 */
vluint64_t main_time = 0;

/**
 * SystemC time stamp callback function.
 *
 * Returns the current simulation time in SystemC units. Required by
 * Verilator for SystemC-style simulation integration. The time value
 * is derived from main_time, which increments with each clock cycle.
 *
 * @return Current simulation time as a double-precision value.
 */
double sc_time_stamp() { return main_time; }

/**
 * System-on-Chip simulation wrapper class.
 *
 * Encapsulates the Verilator-generated top-level module and provides
 * methods for clock generation, reset control, and bus transactions.
 * Manages the simulation state machine and coordinates between the
 * Verilator evaluation calls and the TCP command interface.
 */
class SoC {
public:
  /** Verilator-generated top-level module instance. */
  std::unique_ptr<Vtop_soc> top;

  /**
   * Constructs and initializes the SoC simulation.
   *
   * Creates the Verilator module instance, applies reset sequence (assert
   * reset for one clock cycle, then deassert), and prepares the simulation
   * for normal operation. The reset sequence ensures all state machines
   * and registers start in known initial states.
   */
  SoC() {
    top = std::make_unique<Vtop_soc>();
    top->clk = 0;
    top->rst_n = 0;
    tick();
    top->rst_n = 1;
    tick();
  }

  /**
   * Advances simulation by one clock cycle.
   *
   * Generates a complete clock cycle by setting clk high, evaluating the
   * Verilator model, then setting clk low and evaluating again. This two-phase
   * evaluation ensures proper setup and hold time behavior for sequential
   * logic. Increments main_time to track simulation progress.
   */
  void tick() {
    top->clk = 1;
    top->eval();
    main_time++;
    top->clk = 0;
    top->eval();
    main_time++;
  }

  /**
   * Performs a bus write transaction to the specified address.
   *
   * Asserts chip select and write enable, drives address and data signals,
   * then ticks the clock to complete the transaction. The write operation
   * targets memory-mapped registers in the quantum hardware peripherals.
   *
   * @param addr 32-bit memory-mapped address to write
   * @param data 32-bit value to write
   */
  void write(uint32_t addr, uint32_t data) {
    top->bus_cs = 1;
    top->bus_we = 1;
    top->bus_addr = addr;
    top->bus_wdata = data;
    tick();
    top->bus_cs = 0;
    top->bus_we = 0;
  }

  /**
   * Performs a bus read transaction from the specified address.
   *
   * Asserts chip select with write enable deasserted, drives the address
   * signal, then ticks the clock to capture the read data. Returns the
   * value read from the memory-mapped register.
   *
   * @param addr 32-bit memory-mapped address to read
   * @return 32-bit value read from the address
   */
  uint32_t read(uint32_t addr) {
    top->bus_cs = 1;
    top->bus_we = 0;
    top->bus_addr = addr;
    tick();
    uint32_t data = top->bus_rdata;
    top->bus_cs = 0;
    return data;
  }
};

/**
 * @defgroup Protocol Protocol Command Definitions
 * @brief Binary protocol opcodes for TCP communication.
 *
 * The protocol uses a simple command format: 1-byte opcode followed by
 * optional 4-byte address and/or 4-byte data fields. All multi-byte
 * values are transmitted in little-endian byte order.
 * @{
 */
#define CMD_STEP 0x01  /**< Step simulation by N clock cycles */
#define CMD_WRITE 0x02 /**< Write data to memory-mapped address */
#define CMD_READ 0x03  /**< Read data from memory-mapped address */
#define CMD_EXIT 0xFF  /**< Exit simulation and close connection */
/** @} */

/**
 * Main entry point for Verilator simulation server.
 *
 * Initializes the Verilator simulation, sets up a TCP server socket on port
 * 8000, and enters a command processing loop. Accepts connections from the
 * Rust host controller and processes commands to step the simulation and
 * access hardware registers. The loop continues until the connection is
 * closed or an exit command is received.
 *
 * @param argc Command-line argument count
 * @param argv Command-line argument vector (passed to Verilator)
 * @return Exit status code (0 on success)
 */
int main(int argc, char **argv) {
  Verilated::commandArgs(argc, argv);
  SoC soc;

  /**
   * Setup TCP server socket for host controller communication.
   *
   * Creates a TCP socket, binds to port 8000, and listens for incoming
   * connections. The server accepts a single connection and processes
   * commands until the connection is closed.
   */
  int server_fd, new_socket;
  struct sockaddr_in address;
  int opt = 1;
  int addrlen = sizeof(address);

  if ((server_fd = socket(AF_INET, SOCK_STREAM, 0)) == 0) {
    perror("socket failed");
    exit(EXIT_FAILURE);
  }

  if (setsockopt(server_fd, SOL_SOCKET, SO_REUSEADDR | SO_REUSEPORT, &opt,
                 sizeof(opt))) {
    perror("setsockopt");
    exit(EXIT_FAILURE);
  }

  address.sin_family = AF_INET;
  address.sin_addr.s_addr = INADDR_ANY;
  address.sin_port = htons(8000);

  if (bind(server_fd, (struct sockaddr *)&address, sizeof(address)) < 0) {
    perror("bind failed");
    exit(EXIT_FAILURE);
  }

  if (listen(server_fd, 3) < 0) {
    perror("listen");
    exit(EXIT_FAILURE);
  }

  printf("[HW-SRV] Physics Engine listening on port 8000...\n");
  printf("[HW-SRV] Waiting for Rust Host Controller...\n");

  if ((new_socket = accept(server_fd, (struct sockaddr *)&address,
                           (socklen_t *)&addrlen)) < 0) {
    perror("accept");
    exit(EXIT_FAILURE);
  }

  printf("[HW-SRV] Controller Connected! Starting Simulation Loop.\n");

  /**
   * Command processing loop.
   *
   * Continuously reads commands from the TCP connection and executes
   * corresponding operations: step simulation, read/write registers, or
   * exit. Each command consists of a 1-byte opcode followed by optional
   * address and data fields.
   */
  uint8_t buffer[1024];
  bool running = true;

  while (running) {
    int valread = read(new_socket, buffer, 1);
    if (valread <= 0)
      break;

    uint8_t cmd = buffer[0];
    uint32_t addr = 0;
    uint32_t data = 0;
    uint32_t response = 0;

    switch (cmd) {
    case CMD_STEP:
      read(new_socket, &data, 4);
      for (uint32_t i = 0; i < data; i++)
        soc.tick();
      send(new_socket, &response, 4, 0);
      break;

    case CMD_WRITE:
      read(new_socket, &addr, 4);
      read(new_socket, &data, 4);
      soc.write(addr, data);
      send(new_socket, &response, 4, 0);
      break;

    case CMD_READ:
      read(new_socket, &addr, 4);
      response = soc.read(addr);
      send(new_socket, &response, 4, 0);
      break;

    case CMD_EXIT:
      running = false;
      break;
    }
  }

  printf("[HW-SRV] Simulation Closed.\n");
  close(new_socket);
  close(server_fd);
  return 0;
}
