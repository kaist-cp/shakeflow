#![allow(incomplete_features)]
#![feature(generic_const_exprs)]
#![feature(adt_const_params)]
#![allow(type_alias_bounds)]
#![allow(clippy::type_complexity)]

use std::path::Path;

use shakeflow::*;
use shakeflow_std::*;

pub mod bsg_dataflow;
pub mod bsg_noc;

fn main() -> Result<(), PackageError> {
    let mut package = Package::default();

    package.add(bsg_dataflow::bsg_1_to_n_tagged::m::<bool, 10>());
    package.add(bsg_dataflow::bsg_1_to_n_tagged_fifo::m::<Bits<U<10>>, 5, 5, 0b00000, false, false>());
    package.add(bsg_dataflow::bsg_1_to_n_tagged_fifo_shared::m());
    package.add(bsg_dataflow::bsg_8b10b_decode_comb::m());
    package.add(bsg_dataflow::bsg_8b10b_encode_comb::m());
    package.add(bsg_dataflow::bsg_8b10b_shift_decoder::m());
    package.add(bsg_dataflow::bsg_channel_tunnel::m::<U<10>, 5, 3>());
    package.add(bsg_dataflow::bsg_channel_tunnel_in::m::<U<10>, 5, 3>());
    package.add(bsg_dataflow::bsg_channel_tunnel_out::m::<U<10>, 5, 3>());
    package.add(bsg_dataflow::bsg_channel_tunnel_wormhole::m());
    package.add(bsg_dataflow::bsg_compare_and_swap::m::<U<10>, false>());
    package.add(bsg_dataflow::bsg_credit_to_token::m::<Bits<U<10>>, 4>());
    package.add(bsg_dataflow::bsg_fifo_1r1w_large::m::<Bits<U<10>>, 4>());
    package.add(bsg_dataflow::bsg_fifo_1r1w_large_banked::m::<Bits<U<10>>, 4>());
    package.add(bsg_dataflow::bsg_fifo_1r1w_narrowed::m::<U<10>, U<3>, 5, true>());
    package.add(bsg_dataflow::bsg_fifo_1r1w_pseudo_large::m::<Bits<U<10>>, 5>());
    package.add(bsg_dataflow::bsg_fifo_1r1w_small::m::<Bits<U<5>>, 5, false>());
    package.add(bsg_dataflow::bsg_fifo_1r1w_small_credit_on_input::m::<Bits<U<10>>, 5, false>());
    package.add(bsg_dataflow::bsg_fifo_1r1w_small_hardened::m::<Bits<U<5>>, 5>());
    package.add(bsg_dataflow::bsg_fifo_1r1w_small_unhardened::m::<Bits<U<5>>, 5>());
    package.add(bsg_dataflow::bsg_fifo_1rw_large::m::<Bits<U<10>>, 5>());
    package.add(bsg_dataflow::bsg_fifo_bypass::m::<Bits<U<10>>>(bsg_dataflow::bsg_fifo_1r1w_small::m::<_, 5, false>()));
    package.add(bsg_dataflow::bsg_fifo_reorder::m());
    package.add(bsg_dataflow::bsg_fifo_shift_datapath::m::<Bits<U<10>>, 5>());
    package.add(bsg_dataflow::bsg_fifo_tracker::m::<10, U<4>>());
    package.add(bsg_dataflow::bsg_flatten_2d_array::m::<U<10>, 5>());
    package.add(bsg_dataflow::bsg_flow_counter::m::<Bits<U<10>>, 5, false>(bsg_dataflow::bsg_one_fifo::m()));
    package.add(bsg_dataflow::bsg_make_2d_array::m::<U<10>, 5>());
    package.add(bsg_dataflow::bsg_one_fifo::m::<U<10>, { Protocol::Helpful }>());
    package.add(bsg_dataflow::bsg_parallel_in_serial_out::m::<Bits<U<10>>, 5, false>());
    package.add(bsg_dataflow::bsg_parallel_in_serial_out_dynamic::m::<Bits<U<10>>, U<5>>());
    package.add(bsg_dataflow::bsg_parallel_in_serial_out_passthrough::m::<Bits<U<10>>, 5>());
    package.add(bsg_dataflow::bsg_permute_box::m::<U<10>, 5>());
    package.add(bsg_dataflow::bsg_ready_to_credit_flow_converter::m::<Bits<U<10>>, 10, 5>());
    package.add(bsg_dataflow::bsg_relay_fifo::m::<U<10>>());
    package.add(bsg_dataflow::bsg_round_robin_1_to_n::m::<Bits<U<10>>, 5>());
    package.add(bsg_dataflow::bsg_round_robin_2_to_2::m::<Bits<U<10>>>());
    package.add(bsg_dataflow::bsg_round_robin_fifo_to_fifo::m());
    package.add(bsg_dataflow::bsg_round_robin_n_to_1::m::<Bits<U<10>>, 5, true>());
    package.add(bsg_dataflow::bsg_rr_f2f_input::m());
    package.add(bsg_dataflow::bsg_rr_f2f_middle::m());
    package.add(bsg_dataflow::bsg_rr_f2f_output::m());
    package.add(bsg_dataflow::bsg_sbox::m::<Bits<U<10>>, 5, false, false>());
    package.add(bsg_dataflow::bsg_scatter_gather::m::<5>());
    package.add(bsg_dataflow::bsg_serial_in_parallel_out::m::<Bits<U<10>>, 5>());
    package.add(bsg_dataflow::bsg_serial_in_parallel_out_dynamic::m::<Bits<U<10>>, U<5>>());
    package.add(bsg_dataflow::bsg_serial_in_parallel_out_full::m::<Bits<U<10>>, 5, false, false>());
    package.add(bsg_dataflow::bsg_serial_in_parallel_out_passthrough::m::<Bits<U<10>>, 5>());
    package.add(bsg_dataflow::bsg_shift_reg::m::<U<10>, 5>());
    package.add(bsg_dataflow::bsg_sort_4::m::<U<10>>());
    package.add(bsg_dataflow::bsg_sort_4::m::<U<10>>());
    package.add(bsg_dataflow::bsg_two_buncher::m());

    package.add(bsg_noc::bsg_barrier::m::<4>());
    package.add(bsg_noc::bsg_mesh_router::m::<10, 4, 4, 1, 1, 2, 5, false, false>());
    package.add(bsg_noc::bsg_mesh_router_buffered::m::<10, 4, 4, 1, 1, 2, 5, false, false>([false; 5]));
    package.add(bsg_noc::bsg_mesh_router_decoder_dor::m::<8, 8, 2, 0, 0, true, true>([false; 9]));
    package.add(bsg_noc::bsg_mesh_stitch::m::<10, 4, 4, 1>());
    package.add(bsg_noc::bsg_mesh_to_ring_stitch::m::<10, 5>());
    package.add(bsg_noc::bsg_noc_repeater_node::m::<Bits<U<10>>>());
    package.add(bsg_noc::bsg_router_crossbar_o_by_i::m::<Bits<U<10>>, 5, 3, false>());
    package.add(bsg_noc::bsg_wormhole_concentrator::m::<20, 5, 4, 10, 5>());
    package.add(bsg_noc::bsg_wormhole_concentrator_out::m::<20, 5, 4, 10, 5>());
    package.add(bsg_noc::bsg_wormhole_concentrator_in::m::<20, 5, 4, 10, 5>());
    package.add(bsg_noc::bsg_wormhole_router::m());
    package.add(bsg_noc::bsg_wormhole_router_adapter::m::<8, 5, 3, 8>());
    package.add(bsg_noc::bsg_wormhole_router_adapter_in::m::<8, 5, 3, 8>());
    package.add(bsg_noc::bsg_wormhole_router_adapter_out::m::<8, 5, 3, 8>());
    package.add(bsg_noc::bsg_wormhole_router_decoder_dor::m::<false>());
    package.add(bsg_noc::bsg_wormhole_router_input_control::m::<10, 5>());
    package.add(bsg_noc::bsg_wormhole_router_output_control::m::<10, 4>());

    package.gen_vir(Path::new("./build"))
}
