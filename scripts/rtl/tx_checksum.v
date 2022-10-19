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
 * Transmit checksum offload module
 */
module tx_checksum #
(
    // Width of AXI stream interfaces in bits
    parameter DATA_WIDTH = 256,
    // AXI stream tkeep signal width (words per cycle)
    parameter KEEP_WIDTH = (DATA_WIDTH/8),
    // Propagate tid signal
    parameter ID_ENABLE = 0,
    // tid signal width
    parameter ID_WIDTH = 8,
    // Propagate tdest signal
    parameter DEST_ENABLE = 0,
    // tdest signal width
    parameter DEST_WIDTH = 8,
    // Propagate tuser signal
    parameter USER_ENABLE = 1,
    // tuser signal width
    parameter USER_WIDTH = 1,
    // Use checksum init value
    parameter USE_INIT_VALUE = 0,
    // Depth of data FIFO in words
    parameter DATA_FIFO_DEPTH = 4096,
    // Depth of checksum FIFO
    parameter CHECKSUM_FIFO_DEPTH = 64
)
(
    input  wire                   clk,
    input  wire                   rst,

    /*
     * AXI input
     */
    input  wire [DATA_WIDTH-1:0]  s_axis_tdata,
    input  wire [KEEP_WIDTH-1:0]  s_axis_tkeep,
    input  wire                   s_axis_tvalid,
    output wire                   s_axis_tready,
    input  wire                   s_axis_tlast,
    // input  wire [ID_WIDTH-1:0]    s_axis_tid,
    // input  wire [DEST_WIDTH-1:0]  s_axis_tdest,
    input  wire [USER_WIDTH-1:0]  s_axis_tuser,

    /*
     * AXI output
     */
    output wire [DATA_WIDTH-1:0]  m_axis_tdata,
    output wire [KEEP_WIDTH-1:0]  m_axis_tkeep,
    output wire                   m_axis_tvalid,
    input  wire                   m_axis_tready,
    output wire                   m_axis_tlast,
    // output wire [ID_WIDTH-1:0]    m_axis_tid,
    // output wire [DEST_WIDTH-1:0]  m_axis_tdest,
    output wire [USER_WIDTH-1:0]  m_axis_tuser,

    /*
     * Control
     */
    input  wire                   s_axis_cmd_csum_enable,
    input  wire [7:0]             s_axis_cmd_csum_start,
    input  wire [7:0]             s_axis_cmd_csum_offset,
    input  wire [15:0]            s_axis_cmd_csum_init,
    input  wire                   s_axis_cmd_valid,
    output wire                   s_axis_cmd_ready
);

// check configuration
initial begin
    if (DATA_WIDTH != 512) begin $error; $finish; end
    if (KEEP_WIDTH != 64) begin $error; $finish; end
    if (ID_ENABLE != 0) begin $error; $finish; end
    if (ID_WIDTH != 8) begin $error; $finish; end
    if (DEST_ENABLE != 0) begin $error; $finish; end
    if (DEST_WIDTH != 8) begin $error; $finish; end
    if (USER_ENABLE != 1) begin $error; $finish; end
    if (USER_WIDTH != 1) begin $error; $finish; end
    if (USE_INIT_VALUE != 1) begin $error; $finish; end
    if (DATA_FIFO_DEPTH != 16384) begin $error; $finish; end
    if (CHECKSUM_FIFO_DEPTH != 4) begin $error; $finish; end
end

// TID and TDEST are disabled
tx_checksum_inner tx_checksum_inst (
    .clk(clk),
    .rst(rst),

    .s_axis_tdata(s_axis_tdata),
    .s_axis_tkeep(s_axis_tkeep),
    .s_axis_tvalid(s_axis_tvalid),
    .s_axis_tready(s_axis_tready),
    .s_axis_tlast(s_axis_tlast),
    // .s_axis_tid(s_axis_tid),
    // .s_axis_tdest(s_axis_tdest),
    .s_axis_tuser(s_axis_tuser),

    .m_axis_tdata(m_axis_tdata),
    .m_axis_tkeep(m_axis_tkeep),
    .m_axis_tvalid(m_axis_tvalid),
    .m_axis_tready(m_axis_tready),
    .m_axis_tlast(m_axis_tlast),
    // .m_axis_tid(m_axis_tid),
    // .m_axis_tdest(m_axis_tdest),
    .m_axis_tuser(m_axis_tuser),

    .s_axis_cmd_csum_enable(s_axis_cmd_csum_enable),
    .s_axis_cmd_csum_start(s_axis_cmd_csum_start),
    .s_axis_cmd_csum_offset(s_axis_cmd_csum_offset),
    .s_axis_cmd_csum_init(s_axis_cmd_csum_init),
    .s_axis_cmd_valid(s_axis_cmd_valid),
    .s_axis_cmd_ready(s_axis_cmd_ready)
);

endmodule
