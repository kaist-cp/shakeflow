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
 * Descriptor operation mux
 */
module desc_op_mux #
(
    // Number of ports
    parameter PORTS = 2,
    // Select field width
    parameter SELECT_WIDTH = 1,
    // Queue index width
    parameter QUEUE_INDEX_WIDTH = 4,
    // Queue element pointer width
    parameter QUEUE_PTR_WIDTH = 16,
    // Completion queue index width
    parameter CPL_QUEUE_INDEX_WIDTH = 4,
    // Input request tag field width
    parameter S_REQ_TAG_WIDTH = 8,
    // Output request tag field width (towards descriptor module)
    // Additional bits required for response routing
    parameter M_REQ_TAG_WIDTH = S_REQ_TAG_WIDTH+$clog2(PORTS),
    // Width of AXI stream interface in bits
    parameter AXIS_DATA_WIDTH = 256,
    // AXI stream tkeep signal width (words per cycle)
    parameter AXIS_KEEP_WIDTH = AXIS_DATA_WIDTH/8,
    // select round robin arbitration
    parameter ARB_TYPE_ROUND_ROBIN = 0,
    // LSB priority selection
    parameter ARB_LSB_HIGH_PRIORITY = 1
)
(
    input  wire                                   clk,
    input  wire                                   rst,

    /*
     * Descriptor request output (to descriptor module)
     */
    output wire [SELECT_WIDTH-1:0]                m_axis_req_sel,
    output wire [QUEUE_INDEX_WIDTH-1:0]           m_axis_req_queue,
    output wire [M_REQ_TAG_WIDTH-1:0]             m_axis_req_tag,
    output wire                                   m_axis_req_valid,
    input  wire                                   m_axis_req_ready,

    /*
     * Descriptor request status input (from descriptor module)
     */
    input  wire [QUEUE_INDEX_WIDTH-1:0]           s_axis_req_status_queue,
    input  wire [QUEUE_PTR_WIDTH-1:0]             s_axis_req_status_ptr,
    input  wire [CPL_QUEUE_INDEX_WIDTH-1:0]       s_axis_req_status_cpl,
    input  wire [M_REQ_TAG_WIDTH-1:0]             s_axis_req_status_tag,
    input  wire                                   s_axis_req_status_empty,
    input  wire                                   s_axis_req_status_error,
    input  wire                                   s_axis_req_status_valid,

    /*
     * Descriptor data input (from descriptor module)
     */
    input  wire [AXIS_DATA_WIDTH-1:0]             s_axis_desc_tdata,
    input  wire [AXIS_KEEP_WIDTH-1:0]             s_axis_desc_tkeep,
    input  wire                                   s_axis_desc_tvalid,
    output wire                                   s_axis_desc_tready,
    input  wire                                   s_axis_desc_tlast,
    input  wire [M_REQ_TAG_WIDTH-1:0]             s_axis_desc_tid,
    input  wire                                   s_axis_desc_tuser,

    /*
     * Descriptor request input
     */
    input  wire [PORTS*SELECT_WIDTH-1:0]          s_axis_req_sel,
    input  wire [PORTS*QUEUE_INDEX_WIDTH-1:0]     s_axis_req_queue,
    input  wire [PORTS*S_REQ_TAG_WIDTH-1:0]       s_axis_req_tag,
    input  wire [PORTS-1:0]                       s_axis_req_valid,
    output wire [PORTS-1:0]                       s_axis_req_ready,

    /*
     * Descriptor request status output
     */
    output wire [PORTS*QUEUE_INDEX_WIDTH-1:0]     m_axis_req_status_queue,
    output wire [PORTS*QUEUE_PTR_WIDTH-1:0]       m_axis_req_status_ptr,
    output wire [PORTS*CPL_QUEUE_INDEX_WIDTH-1:0] m_axis_req_status_cpl,
    output wire [PORTS*S_REQ_TAG_WIDTH-1:0]       m_axis_req_status_tag,
    output wire [PORTS-1:0]                       m_axis_req_status_empty,
    output wire [PORTS-1:0]                       m_axis_req_status_error,
    output wire [PORTS-1:0]                       m_axis_req_status_valid,

    /*
     * Descriptor data output
     */
    output wire [PORTS*AXIS_DATA_WIDTH-1:0]       m_axis_desc_tdata,
    output wire [PORTS*AXIS_KEEP_WIDTH-1:0]       m_axis_desc_tkeep,
    output wire [PORTS-1:0]                       m_axis_desc_tvalid,
    input  wire [PORTS-1:0]                       m_axis_desc_tready,
    output wire [PORTS-1:0]                       m_axis_desc_tlast,
    output wire [PORTS*S_REQ_TAG_WIDTH-1:0]       m_axis_desc_tid,
    output wire [PORTS-1:0]                       m_axis_desc_tuser
);

initial begin
    if (PORTS != 2) begin $error; $finish; end
    if (SELECT_WIDTH != 1) begin $error; $finish; end
    if (QUEUE_INDEX_WIDTH != 13) begin $error; $finish; end
    if (QUEUE_PTR_WIDTH != 16) begin $error; $finish; end
    if (CPL_QUEUE_INDEX_WIDTH != 13) begin $error; $finish; end
    if (S_REQ_TAG_WIDTH != 5) begin $error; $finish; end
    if (M_REQ_TAG_WIDTH != 6) begin $error; $finish; end
    if (AXIS_DATA_WIDTH != 128) begin $error; $finish; end
    if (AXIS_KEEP_WIDTH != 16) begin $error; $finish; end
    if (ARB_TYPE_ROUND_ROBIN != 1) begin $error; $finish; end
    if (ARB_LSB_HIGH_PRIORITY != 1) begin $error; $finish; end
end

desc_op_mux_inner desc_op_mux_inst (
    .clk(clk),
    .rst(rst),

    .m_axis_req_sel(m_axis_req_sel),
    .m_axis_req_queue(m_axis_req_queue),
    .m_axis_req_tag(m_axis_req_tag),
    .m_axis_req_valid(m_axis_req_valid),
    .m_axis_req_ready(m_axis_req_ready),

    .s_axis_req_status_queue(s_axis_req_status_queue),
    .s_axis_req_status_ptr(s_axis_req_status_ptr),
    .s_axis_req_status_cpl(s_axis_req_status_cpl),
    .s_axis_req_status_tag(s_axis_req_status_tag),
    .s_axis_req_status_empty(s_axis_req_status_empty),
    .s_axis_req_status_error(s_axis_req_status_error),
    .s_axis_req_status_valid(s_axis_req_status_valid),

    .s_axis_desc_tdata(s_axis_desc_tdata),
    .s_axis_desc_tkeep(s_axis_desc_tkeep),
    .s_axis_desc_tvalid(s_axis_desc_tvalid),
    .s_axis_desc_tready(s_axis_desc_tready),
    .s_axis_desc_tlast(s_axis_desc_tlast),
    .s_axis_desc_tid(s_axis_desc_tid),
    .s_axis_desc_tuser(s_axis_desc_tuser),

    .s_axis_req_sel(s_axis_req_sel),
    .s_axis_req_queue(s_axis_req_queue),
    .s_axis_req_tag(s_axis_req_tag),
    .s_axis_req_valid(s_axis_req_valid),
    .s_axis_req_ready(s_axis_req_ready),

    .m_axis_req_status_queue(m_axis_req_status_queue),
    .m_axis_req_status_ptr(m_axis_req_status_ptr),
    .m_axis_req_status_cpl(m_axis_req_status_cpl),
    .m_axis_req_status_tag(m_axis_req_status_tag),
    .m_axis_req_status_empty(m_axis_req_status_empty),
    .m_axis_req_status_error(m_axis_req_status_error),
    .m_axis_req_status_valid(m_axis_req_status_valid),
    
    .m_axis_desc_tdata(m_axis_desc_tdata),
    .m_axis_desc_tkeep(m_axis_desc_tkeep),
    .m_axis_desc_tvalid(m_axis_desc_tvalid),
    .m_axis_desc_tready(m_axis_desc_tready),
    .m_axis_desc_tlast(m_axis_desc_tlast),
    .m_axis_desc_tid(m_axis_desc_tid),
    .m_axis_desc_tuser(m_axis_desc_tuser)
);

endmodule
