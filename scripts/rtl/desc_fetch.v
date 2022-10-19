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
 * Descriptor fetch module
 */
module desc_fetch #
(
    // Number of ports
    parameter PORTS = 2,
    // Select field width
    parameter SELECT_WIDTH = $clog2(PORTS),
    // RAM segment count
    parameter SEG_COUNT = 2,
    // RAM segment data width
    parameter SEG_DATA_WIDTH = 64,
    // RAM segment address width
    parameter SEG_ADDR_WIDTH = 8,
    // RAM segment byte enable width
    parameter SEG_BE_WIDTH = SEG_DATA_WIDTH/8,
    // RAM address width
    parameter RAM_ADDR_WIDTH = SEG_ADDR_WIDTH+$clog2(SEG_COUNT)+$clog2(SEG_BE_WIDTH),
    // DMA RAM pipeline stages
    parameter RAM_PIPELINE = 2,
    // DMA address width
    parameter DMA_ADDR_WIDTH = 64,
    // DMA length field width
    parameter DMA_LEN_WIDTH = 20,
    // DMA tag field width
    parameter DMA_TAG_WIDTH = 8,
    // Transmit request tag field width
    parameter REQ_TAG_WIDTH = 8,
    // Queue request tag field width
    parameter QUEUE_REQ_TAG_WIDTH = 8,
    // Queue operation tag field width
    parameter QUEUE_OP_TAG_WIDTH = 8,
    // Queue index width
    parameter QUEUE_INDEX_WIDTH = 4,
    // Completion queue index width
    parameter CPL_QUEUE_INDEX_WIDTH = 4,
    // Queue element pointer width
    parameter QUEUE_PTR_WIDTH = 16,
    // Descriptor size (in bytes)
    parameter DESC_SIZE = 16,
    // Log desc block size field width
    parameter LOG_BLOCK_SIZE_WIDTH = 2,
    // Descriptor table size (number of in-flight operations)
    parameter DESC_TABLE_SIZE = 8,
    // Width of AXI stream interface in bits
    parameter AXIS_DATA_WIDTH = DESC_SIZE*8,
    // AXI stream tkeep signal width (words per cycle)
    parameter AXIS_KEEP_WIDTH = AXIS_DATA_WIDTH/8
)
(
    input  wire                                   clk,
    input  wire                                   rst,

    /*
     * Descriptor read request input
     */
    input  wire [SELECT_WIDTH-1:0]                s_axis_req_sel,
    input  wire [QUEUE_INDEX_WIDTH-1:0]           s_axis_req_queue,
    input  wire [REQ_TAG_WIDTH-1:0]               s_axis_req_tag,
    input  wire                                   s_axis_req_valid,
    output wire                                   s_axis_req_ready,

    /*
     * Descriptor read request status output
     */
    output wire [QUEUE_INDEX_WIDTH-1:0]           m_axis_req_status_queue,
    output wire [QUEUE_PTR_WIDTH-1:0]             m_axis_req_status_ptr,
    output wire [CPL_QUEUE_INDEX_WIDTH-1:0]       m_axis_req_status_cpl,
    output wire [REQ_TAG_WIDTH-1:0]               m_axis_req_status_tag,
    output wire                                   m_axis_req_status_empty,
    output wire                                   m_axis_req_status_error,
    output wire                                   m_axis_req_status_valid,

    /*
     * Descriptor data output
     */
    output wire [AXIS_DATA_WIDTH-1:0]             m_axis_desc_tdata,
    output wire [AXIS_KEEP_WIDTH-1:0]             m_axis_desc_tkeep,
    output wire                                   m_axis_desc_tvalid,
    input  wire                                   m_axis_desc_tready,
    output wire                                   m_axis_desc_tlast,
    output wire [REQ_TAG_WIDTH-1:0]               m_axis_desc_tid,
    output wire                                   m_axis_desc_tuser,

    /*
     * Descriptor dequeue request output
     */
    output wire [PORTS*QUEUE_INDEX_WIDTH-1:0]     m_axis_desc_dequeue_req_queue,
    output wire [PORTS*REQ_TAG_WIDTH-1:0]         m_axis_desc_dequeue_req_tag,
    output wire [PORTS-1:0]                       m_axis_desc_dequeue_req_valid,
    input  wire [PORTS-1:0]                       m_axis_desc_dequeue_req_ready,

    /*
     * Descriptor dequeue response input
     */
    input  wire [PORTS*QUEUE_INDEX_WIDTH-1:0]     s_axis_desc_dequeue_resp_queue,
    input  wire [PORTS*QUEUE_PTR_WIDTH-1:0]       s_axis_desc_dequeue_resp_ptr,
    input  wire [PORTS*DMA_ADDR_WIDTH-1:0]        s_axis_desc_dequeue_resp_addr,
    input  wire [PORTS*LOG_BLOCK_SIZE_WIDTH-1:0]  s_axis_desc_dequeue_resp_block_size,
    input  wire [PORTS*CPL_QUEUE_INDEX_WIDTH-1:0] s_axis_desc_dequeue_resp_cpl,
    input  wire [PORTS*QUEUE_REQ_TAG_WIDTH-1:0]   s_axis_desc_dequeue_resp_tag,
    input  wire [PORTS*QUEUE_OP_TAG_WIDTH-1:0]    s_axis_desc_dequeue_resp_op_tag,
    input  wire [PORTS-1:0]                       s_axis_desc_dequeue_resp_empty,
    input  wire [PORTS-1:0]                       s_axis_desc_dequeue_resp_error,
    input  wire [PORTS-1:0]                       s_axis_desc_dequeue_resp_valid,
    output wire [PORTS-1:0]                       s_axis_desc_dequeue_resp_ready,

    /*
     * Descriptor dequeue commit output
     */
    output wire [PORTS*QUEUE_OP_TAG_WIDTH-1:0]    m_axis_desc_dequeue_commit_op_tag,
    output wire [PORTS-1:0]                       m_axis_desc_dequeue_commit_valid,
    input  wire [PORTS-1:0]                       m_axis_desc_dequeue_commit_ready,

    /*
     * DMA read descriptor output
     */
    output wire [DMA_ADDR_WIDTH-1:0]              m_axis_dma_read_desc_dma_addr,
    output wire [RAM_ADDR_WIDTH-1:0]              m_axis_dma_read_desc_ram_addr,
    output wire [DMA_LEN_WIDTH-1:0]               m_axis_dma_read_desc_len,
    output wire [DMA_TAG_WIDTH-1:0]               m_axis_dma_read_desc_tag,
    output wire                                   m_axis_dma_read_desc_valid,
    input  wire                                   m_axis_dma_read_desc_ready,

    /*
     * DMA read descriptor status input
     */
    input  wire [DMA_TAG_WIDTH-1:0]               s_axis_dma_read_desc_status_tag,
    input  wire [3:0]                             s_axis_dma_read_desc_status_error,
    input  wire                                   s_axis_dma_read_desc_status_valid,

    /*
     * RAM interface
     */
    input  wire [SEG_COUNT*SEG_BE_WIDTH-1:0]      dma_ram_wr_cmd_be,
    input  wire [SEG_COUNT*SEG_ADDR_WIDTH-1:0]    dma_ram_wr_cmd_addr,
    input  wire [SEG_COUNT*SEG_DATA_WIDTH-1:0]    dma_ram_wr_cmd_data,
    input  wire [SEG_COUNT-1:0]                   dma_ram_wr_cmd_valid,
    output wire [SEG_COUNT-1:0]                   dma_ram_wr_cmd_ready,
    output wire [SEG_COUNT-1:0]                   dma_ram_wr_done,

    /*
     * Configuration
     */
    input  wire                                   enable
);

initial begin
    if (PORTS != 2) begin $error; $finish; end
    if (SELECT_WIDTH != 1) begin $error; $finish; end
    if (SEG_COUNT != 2) begin $error; $finish; end
    if (SEG_DATA_WIDTH != 512) begin $error; $finish; end
    if (SEG_ADDR_WIDTH != 12) begin $error; $finish; end
    if (SEG_BE_WIDTH != 64) begin $error; $finish; end
    if (RAM_ADDR_WIDTH != 19) begin $error; $finish; end
    if (RAM_PIPELINE != 2) begin $error; $finish; end
    if (DMA_ADDR_WIDTH != 64) begin $error; $finish; end
    if (DMA_LEN_WIDTH != 16) begin $error; $finish; end
    if (DMA_TAG_WIDTH != 14) begin $error; $finish; end
    if (REQ_TAG_WIDTH != 7) begin $error; $finish; end
    if (QUEUE_REQ_TAG_WIDTH != 7) begin $error; $finish; end
    if (QUEUE_OP_TAG_WIDTH != 6) begin $error; $finish; end
    if (QUEUE_INDEX_WIDTH != 13) begin $error; $finish; end
    if (CPL_QUEUE_INDEX_WIDTH != 13) begin $error; $finish; end
    if (QUEUE_PTR_WIDTH != 16) begin $error; $finish; end
    if (DESC_SIZE != 16) begin $error; $finish; end
    if (LOG_BLOCK_SIZE_WIDTH != 2) begin $error; $finish; end
    if (DESC_TABLE_SIZE != 32) begin $error; $finish; end
    if (AXIS_DATA_WIDTH != 128) begin $error; $finish; end
    if (AXIS_KEEP_WIDTH != 16) begin $error; $finish; end
end

desc_fetch_inner desc_fetch_inst (
    .clk(clk),
    .rst(rst),
    
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
    .m_axis_desc_tuser(m_axis_desc_tuser),

    .m_axis_desc_dequeue_req_queue(m_axis_desc_dequeue_req_queue),
    .m_axis_desc_dequeue_req_tag(m_axis_desc_dequeue_req_tag),
    .m_axis_desc_dequeue_req_valid(m_axis_desc_dequeue_req_valid),
    .m_axis_desc_dequeue_req_ready(m_axis_desc_dequeue_req_ready),
    
    .s_axis_desc_dequeue_resp_queue(s_axis_desc_dequeue_resp_queue),
    .s_axis_desc_dequeue_resp_ptr(s_axis_desc_dequeue_resp_ptr),
    .s_axis_desc_dequeue_resp_addr(s_axis_desc_dequeue_resp_addr),
    .s_axis_desc_dequeue_resp_block_size(s_axis_desc_dequeue_resp_block_size),
    .s_axis_desc_dequeue_resp_cpl(s_axis_desc_dequeue_resp_cpl),
    .s_axis_desc_dequeue_resp_tag(s_axis_desc_dequeue_resp_tag),
    .s_axis_desc_dequeue_resp_op_tag(s_axis_desc_dequeue_resp_op_tag),
    .s_axis_desc_dequeue_resp_empty(s_axis_desc_dequeue_resp_empty),
    .s_axis_desc_dequeue_resp_error(s_axis_desc_dequeue_resp_error),
    .s_axis_desc_dequeue_resp_valid(s_axis_desc_dequeue_resp_valid),
    .s_axis_desc_dequeue_resp_ready(s_axis_desc_dequeue_resp_ready),
    
    .m_axis_desc_dequeue_commit_op_tag(m_axis_desc_dequeue_commit_op_tag),
    .m_axis_desc_dequeue_commit_valid(m_axis_desc_dequeue_commit_valid),
    .m_axis_desc_dequeue_commit_ready(m_axis_desc_dequeue_commit_ready),
    
    .m_axis_dma_read_desc_dma_addr(m_axis_dma_read_desc_dma_addr),
    .m_axis_dma_read_desc_ram_addr(m_axis_dma_read_desc_ram_addr),
    .m_axis_dma_read_desc_len(m_axis_dma_read_desc_len),
    .m_axis_dma_read_desc_tag(m_axis_dma_read_desc_tag),
    .m_axis_dma_read_desc_valid(m_axis_dma_read_desc_valid),
    .m_axis_dma_read_desc_ready(m_axis_dma_read_desc_ready),

    .s_axis_dma_read_desc_status_tag(s_axis_dma_read_desc_status_tag),
    .s_axis_dma_read_desc_status_error(s_axis_dma_read_desc_status_error),
    .s_axis_dma_read_desc_status_valid(s_axis_dma_read_desc_status_valid),
    
    .dma_ram_wr_cmd_be(dma_ram_wr_cmd_be),
    .dma_ram_wr_cmd_addr(dma_ram_wr_cmd_addr),
    .dma_ram_wr_cmd_data(dma_ram_wr_cmd_data),
    .dma_ram_wr_cmd_valid(dma_ram_wr_cmd_valid),
    .dma_ram_wr_cmd_ready(dma_ram_wr_cmd_ready),
    .dma_ram_wr_done(dma_ram_wr_done),

    .enable(enable)
);

endmodule
