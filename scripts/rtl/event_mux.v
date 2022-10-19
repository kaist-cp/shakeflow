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
 * Event mux
 */
module event_mux #
(
    // Number of ports
    parameter PORTS = 2,
    // Queue index width
    parameter QUEUE_INDEX_WIDTH = 4,
    // Event type field width
    parameter EVENT_TYPE_WIDTH = 16,
    // Event source field width
    parameter EVENT_SOURCE_WIDTH = 16,
    // select round robin arbitration
    parameter ARB_TYPE_ROUND_ROBIN = 0,
    // LSB priority selection
    parameter ARB_LSB_HIGH_PRIORITY = 1
)
(
    input  wire                                clk,
    input  wire                                rst,

    /*
     * Event output
     */
    output wire [QUEUE_INDEX_WIDTH-1:0]        m_axis_event_queue,
    output wire [EVENT_TYPE_WIDTH-1:0]         m_axis_event_type,
    output wire [EVENT_SOURCE_WIDTH-1:0]       m_axis_event_source,
    output wire                                m_axis_event_valid,
    input  wire                                m_axis_event_ready,

    /*
     * Event input
     */
    input  wire [PORTS*QUEUE_INDEX_WIDTH-1:0]  s_axis_event_queue,
    input  wire [PORTS*EVENT_TYPE_WIDTH-1:0]   s_axis_event_type,
    input  wire [PORTS*EVENT_SOURCE_WIDTH-1:0] s_axis_event_source,
    input  wire [PORTS-1:0]                    s_axis_event_valid,
    output wire [PORTS-1:0]                    s_axis_event_ready
);

initial begin
    if (PORTS != 2) begin $error; $finish; end
    if (QUEUE_INDEX_WIDTH != 5) begin $error; $finish; end
    if (EVENT_TYPE_WIDTH != 16) begin $error; $finish; end
    if (EVENT_SOURCE_WIDTH != 16) begin $error; $finish; end
    if (ARB_TYPE_ROUND_ROBIN != 1) begin $error; $finish; end
    if (ARB_LSB_HIGH_PRIORITY != 1) begin $error; $finish; end
end

event_mux_inner event_mux_inst (
    .clk(clk),
    .rst(rst),

    .m_axis_event_queue(m_axis_event_queue),
    .m_axis_event_type(m_axis_event_type),
    .m_axis_event_source(m_axis_event_source),
    .m_axis_event_valid(m_axis_event_valid),
    .m_axis_event_ready(m_axis_event_ready),
    
    .s_axis_event_queue(s_axis_event_queue),
    .s_axis_event_type(s_axis_event_type),
    .s_axis_event_source(s_axis_event_source),
    .s_axis_event_valid(s_axis_event_valid),
    .s_axis_event_ready(s_axis_event_ready)
);

endmodule
