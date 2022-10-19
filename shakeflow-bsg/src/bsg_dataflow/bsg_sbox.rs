//! The switchbox concentrates working channel signals to reduce the complexity of downstream logic.
//!
//! The `ONE_HOT` option selectively uses one-hot muxes, pipelining the mux decode logic at the expensive of energy
//! and area. `PIPELINE_INDIR` and `PIPELINE_OUTDIR` add pipelining (two element FIFOs) in each direction.
//!
//! NB: An implementation based on Benes networks could potentially use less area, at the cost of complexity and wire
//! congestion.

use shakeflow::*;
use shakeflow_std::*;

use super::bsg_scatter_gather as scatter_gather;

#[derive(Debug, Interface)]
pub struct IC<V: Signal, const N: usize> {
    calibration_done: UniChannel<bool>,
    channel_active: UniChannel<Bits<U<N>>>,
    #[member(name = "in")]
    inp: [VrChannel<V>; N],
    out_me: [VrChannel<V>; N],
}

#[derive(Debug, Interface)]
pub struct EC<V: Signal, const N: usize> {
    #[member(name = "in")]
    inp: [VrChannel<V>; N],
    out_me: [VrChannel<V>; N],
}

pub fn m<V: Signal, const N: usize, const PIPELINE_INDIR: bool, const PIPELINE_OUTDIR: bool>(
) -> Module<IC<V, N>, EC<V, N>> {
    composite::<IC<V, N>, EC<V, N>, _>("bsg_sbox", Some("i"), Some("o"), |input, k| {
        let IC { calibration_done, channel_active, inp, out_me } = input;

        let scatter_gather =
            channel_active.comb_inline(k, scatter_gather::m::<N>()).buffer(k, scatter_gather::E::new_expr());

        let fwd = scatter_gather.clone().map(k, |input| input.fwd);
        let bk = scatter_gather.map(k, |input| input.bk);

        let inp = inp.permute(k, (fwd.clone(), bk.clone()));
        let out_me = out_me.permute(k, (bk, fwd));

        let inp = if PIPELINE_INDIR {
            inp.array_map_feedback(k, calibration_done.clone(), "infifo", |(ch, calibration_done), k| {
                ch.zip_uni(k, calibration_done)
                    .and_then(k, None, |input| Expr::<Valid<_>>::new(input.1, input.0))
                    .fifo::<2>(k)
            })
        } else {
            inp
        };

        let out_me = if PIPELINE_OUTDIR {
            out_me.array_map_feedback(k, calibration_done, "outfifo", |(ch, calibration_done), k| {
                ch.zip_uni(k, calibration_done)
                    .and_then(k, None, |input| Expr::<Valid<_>>::new(input.1, input.0))
                    .fifo::<2>(k)
            })
        } else {
            out_me
        };

        EC { inp, out_me }
    })
    .build()
}
