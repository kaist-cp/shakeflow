use shakeflow::*;
use shakeflow_std::*;

#[derive(Debug, Clone, Signal)]
pub struct I<const DIRS: usize> {
    data: Bits<U<DIRS>>,
    src_r: Bits<U<DIRS>>,
    dest_r: Bits<Log2<U<DIRS>>>,
}

pub type IC<const DIRS: usize> = UniChannel<I<DIRS>>;
pub type EC<const DIRS: usize> = UniChannel<Bits<U<DIRS>>>;

#[derive(Debug, Clone, Signal)]
pub struct S<const DIRS: usize> {
    activate: bool,
    data: Bits<U<DIRS>>,
    sense: bool,
}

impl<const DIRS: usize> S<DIRS> {
    /// Creates new expr.
    pub fn new_expr() -> Expr<'static, Self> {
        SProj { activate: false.into(), data: 0.into(), sense: false.into() }.into()
    }
}

pub fn m<const DIRS: usize>() -> Module<IC<DIRS>, EC<DIRS>> {
    composite::<IC<DIRS>, EC<DIRS>, _>("bsg_barrier", Some("i"), Some("data_o"), |input, k| {
        input.fsm_map::<S<DIRS>, Bits<U<DIRS>>, _>(k, None, S::new_expr(), |input, state| {
            let data_broadcast_in = state.activate;

            let gather_and = ((!input.src_r) | state.data).all();
            let gather_or = (input.src_r & state.data).any();

            // the barrier should go forward, based on the sense bit, if we are either all 0 or all 1.
            let gather_out = state.sense.cond(gather_or, gather_and);

            // flip sense bit if we are receiving the incoming broadcast
            // we are relying on the P bit still being high at the leaves
            // sense_r  broadcast_in sense_n
            // 0        0            0
            // 0        1            1
            // 1        1            1
            // 1        0            0

            // if we see a transition on data_broadcast_in, then we have completed the barrier
            let sense_next = data_broadcast_in;

            // this is simply a matter of propagating the value in question
            let data_broadcast_out = data_broadcast_in.repeat() & input.src_r;

            // here we propagate the gather_out value, either to network outputs, or to the local activate reg (at the root of the broadcast)
            let dest_decode = Expr::<Bits<Sum<U<DIRS>, U<1>>>>::from(1) << input.dest_r.resize();
            let data_gather_out = dest_decode & gather_out.repeat();

            let output = data_broadcast_out | data_gather_out.clip_const::<U<DIRS>>(0);

            let activate_next = data_gather_out[DIRS];

            let state_next = SProj { activate: activate_next, data: input.data, sense: sense_next }.into();

            (output, state_next)
        })
    })
    .build()
}
