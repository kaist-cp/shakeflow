use shakeflow::*;
use shakeflow_std::*;

pub type IC<V: Signal, const I_ELS: usize> = [VrChannel<V>; I_ELS];
pub type EC<V: Signal, const O_ELS: usize> = [VrChannel<V>; O_ELS];

#[derive(Debug, Clone, Signal)]
pub struct CtlI<const O_ELS: usize> {
    sel_io: Bits<Log2<U<O_ELS>>>,
}

#[derive(Debug, Clone, Signal)]
pub struct CtlE<const I_ELS: usize> {
    grants_oi_one_hot: Bits<Log2<U<I_ELS>>>,
}

pub type CtlIC<const I_ELS: usize, const O_ELS: usize> = [VrChannel<CtlI<O_ELS>>; I_ELS];
pub type CtlEC<const I_ELS: usize, const O_ELS: usize> = [VrChannel<CtlE<I_ELS>>; O_ELS];

pub fn m<V: Signal, const I_ELS: usize, const O_ELS: usize, const USE_CREDITS: bool>(
) -> Module<IC<V, I_ELS>, EC<V, O_ELS>>
where [(); V::WIDTH]: {
    composite::<IC<V, I_ELS>, EC<V, O_ELS>, _>("bsg_router_crossbar_o_by_i", Some("i"), Some("o"), |input, k| {
        let fifo_output = input.array_map(k, "fifo", |ch, k| {
            if USE_CREDITS {
                ch.comb_inline(k, crate::bsg_dataflow::bsg_fifo_1r1w_small_credit_on_input::m::<V, 2, false>())
            } else {
                ch.comb_inline(k, crate::bsg_dataflow::bsg_fifo_1r1w_small::m::<V, 2, false>())
            }
        });

        let (ctl_input, data): (CtlIC<I_ELS, O_ELS>, [UniChannel<V>; I_ELS]) = fifo_output
            .array_map(k, "split_data", |ch, k| {
                let (ch, ch_fwd) = ch.clone_uni(k);
                let control =
                    ch.map(k, |input| CtlIProj { sel_io: input.repr().clip_const::<Log2<U<O_ELS>>>(0) }.into());
                let data = ch_fwd.map(k, |input| input.inner);
                (control, data)
            })
            .unzip();

        let ctl_output = ctl_input.module_inst::<CtlEC<I_ELS, O_ELS>>(
            k,
            "bsg_crossbar_control_basic_o_by_i",
            "ctrl0",
            vec![("i_els_p", I_ELS), ("o_els_p", O_ELS)],
            true,
            Some("i"),
            Some("o"),
        );

        let data = data.concat(k);

        ctl_output.array_map_feedback(k, data, "mux", |(control, data), k| {
            control.zip_uni(k, data).map(k, |input| {
                let (ctl, data) = *input;
                data[ctl.grants_oi_one_hot]
            })
        })
    })
    .build()
}
