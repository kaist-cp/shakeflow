/*

Note:

This file is identical to the original repository's `cpl_op_mux.v` file, 
sans the changed module name. This change was made because the `cpl_op_mux`
is instantiated in both `mqnic_port` and `mqnic_interface`, but with
different module parameters. ShakeFlow currently cannot support different
module parameters, so the two instances must use separate module files.

When ShakeFlow can support module parameters, the following changes should
be made to the repository:
- Change `cpl_op_mux_mqnic_port.v` and `cpl_op_mux_mqnic_interface.v`
  back to the original `cpl_op_mux.v` file,
- Change the instantiations in `mqnic_port.v` and `mqnic_interface.v`
  such that they refer to the original `cpl_op_mux` module,
- Modify the source files specified in `Makefile` and `test_fpga_core.py`
  used for cocotb accordingly.

*/

/*

Copyright 2019, The Regents of the University of California.
All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

   1. Redistributions of source code must retain the above copyright notice,
      this list of conditions and the following disclaimer.

   2. Redistributions in binary form must reproduce the above copyright notice,
      this list of conditions and the following disclaimer in the documentation
      and/or other materials provided with the distribution.

THIS SOFTWARE IS PROVIDED BY THE REGENTS OF THE UNIVERSITY OF CALIFORNIA ''AS
IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE REGENTS OF THE UNIVERSITY OF CALIFORNIA OR
CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT
OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING
IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY
OF SUCH DAMAGE.

The views and conclusions contained in the software and documentation are those
of the authors and should not be interpreted as representing official policies,
either expressed or implied, of The Regents of the University of California.

*/

// Language: Verilog 2001

`timescale 1ns / 1ps

/*
 * Completion operation mux
 */
module cpl_op_mux_mqnic_port #
(
    // Number of ports
    parameter PORTS = 2,
    // Select field width
    parameter SELECT_WIDTH = 1,
    // Queue index width
    parameter QUEUE_INDEX_WIDTH = 4,
    // Input request tag field width
    parameter S_REQ_TAG_WIDTH = 8,
    // Output request tag field width (towards descriptor module)
    // Additional bits required for response routing
    parameter M_REQ_TAG_WIDTH = S_REQ_TAG_WIDTH+$clog2(PORTS),
    // Completion size (bytes)
    parameter CPL_SIZE = 32,
    // select round robin arbitration
    parameter ARB_TYPE_ROUND_ROBIN = 0,
    // LSB priority selection
    parameter ARB_LSB_HIGH_PRIORITY = 1
)
(
    input  wire                                   clk,
    input  wire                                   rst,

    /*
     * Completion request output (to completion module)
     */
    output wire [SELECT_WIDTH-1:0]                m_axis_req_sel,
    output wire [QUEUE_INDEX_WIDTH-1:0]           m_axis_req_queue,
    output wire [M_REQ_TAG_WIDTH-1:0]             m_axis_req_tag,
    output wire [CPL_SIZE*8-1:0]                  m_axis_req_data,
    output wire                                   m_axis_req_valid,
    input  wire                                   m_axis_req_ready,

    /*
     * Completion request status input (from completion module)
     */
    input  wire [M_REQ_TAG_WIDTH-1:0]             s_axis_req_status_tag,
    input  wire                                   s_axis_req_status_full,
    input  wire                                   s_axis_req_status_error,
    input  wire                                   s_axis_req_status_valid,

    /*
     * Completion request input
     */
    input  wire [PORTS*SELECT_WIDTH-1:0]          s_axis_req_sel,
    input  wire [PORTS*QUEUE_INDEX_WIDTH-1:0]     s_axis_req_queue,
    input  wire [PORTS*S_REQ_TAG_WIDTH-1:0]       s_axis_req_tag,
    input  wire [PORTS*CPL_SIZE*8-1:0]            s_axis_req_data,
    input  wire [PORTS-1:0]                       s_axis_req_valid,
    output wire [PORTS-1:0]                       s_axis_req_ready,

    /*
     * Completion request status output
     */
    output wire [PORTS*S_REQ_TAG_WIDTH-1:0]       m_axis_req_status_tag,
    output wire [PORTS-1:0]                       m_axis_req_status_full,
    output wire [PORTS-1:0]                       m_axis_req_status_error,
    output wire [PORTS-1:0]                       m_axis_req_status_valid
);

initial begin
    if (PORTS != 2) begin $error; $finish; end
    if (SELECT_WIDTH != 1) begin $error; $finish; end
    if (QUEUE_INDEX_WIDTH != 13) begin $error; $finish; end
    if (S_REQ_TAG_WIDTH != 5) begin $error; $finish; end
    if (M_REQ_TAG_WIDTH != 6) begin $error; $finish; end
    if (CPL_SIZE != 32) begin $error; $finish; end
    if (ARB_TYPE_ROUND_ROBIN != 1) begin $error; $finish; end
    if (ARB_LSB_HIGH_PRIORITY != 1) begin $error; $finish; end
end

cpl_op_mux_mqnic_port_inner cpl_op_mux_mqnic_port_inst (
    .clk(clk),
    .rst(rst),

    .m_axis_req_sel(m_axis_req_sel),
    .m_axis_req_queue(m_axis_req_queue),
    .m_axis_req_tag(m_axis_req_tag),
    .m_axis_req_data(m_axis_req_data),
    .m_axis_req_valid(m_axis_req_valid),
    .m_axis_req_ready(m_axis_req_ready),

    .s_axis_req_status_tag(s_axis_req_status_tag),
    .s_axis_req_status_full(s_axis_req_status_full),
    .s_axis_req_status_error(s_axis_req_status_error),
    .s_axis_req_status_valid(s_axis_req_status_valid),

    .s_axis_req_sel(s_axis_req_sel),
    .s_axis_req_queue(s_axis_req_queue),
    .s_axis_req_tag(s_axis_req_tag),
    .s_axis_req_data(s_axis_req_data),
    .s_axis_req_valid(s_axis_req_valid),
    .s_axis_req_ready(s_axis_req_ready),

    .m_axis_req_status_tag(m_axis_req_status_tag),
    .m_axis_req_status_full(m_axis_req_status_full),
    .m_axis_req_status_error(m_axis_req_status_error),
    .m_axis_req_status_valid(m_axis_req_status_valid)
);

endmodule
