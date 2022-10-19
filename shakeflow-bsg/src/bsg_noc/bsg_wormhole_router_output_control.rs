use shakeflow::*;
use shakeflow_std::*;

#[derive(Debug, Clone, Signal)]
pub struct I<const FLIT_WIDTH: usize> {
    data: Bits<U<FLIT_WIDTH>>,
    reqs: bool,
    release: bool,
}

pub type IC<const FLIT_WIDTH: usize, const INPUT_DIRS: usize> = [VrChannel<I<FLIT_WIDTH>>; INPUT_DIRS];
pub type EC<const FLIT_WIDTH: usize, const INPUT_DIRS: usize> = VrChannel<(Bits<U<FLIT_WIDTH>>, Bits<U<INPUT_DIRS>>)>;

pub type F<const INPUT_DIRS: usize> = UniChannel<Bits<U<INPUT_DIRS>>>;

pub fn m<const FLIT_WIDTH: usize, const INPUT_DIRS: usize>(
) -> Module<IC<FLIT_WIDTH, INPUT_DIRS>, EC<FLIT_WIDTH, INPUT_DIRS>> {
    composite::<(IC<FLIT_WIDTH, INPUT_DIRS>, F<INPUT_DIRS>), (EC<FLIT_WIDTH, INPUT_DIRS>, F<INPUT_DIRS>), _>(
        "bsg_wormhole_router_output_control",
        Some("i"),
        Some("o"),
        |(input, scheduled), k| {
            let scheduled = scheduled.slice(k);
            let brr_output = input
                .array_zip(scheduled)
                .array_map(k, "brr", |(input, sched), k| {
                    input
                        .zip_uni(k, sched)
                        .and_then(k, None, |input| {
                            let (input, sched) = *input;
                            Expr::<Valid<_>>::new(sched & (!input.release) & input.reqs, input.data)
                        })
                        .buffer(k) // TODO: Remove this buffer
                })
                .arb_mux(k, 1, 0);

            let (brr_output, brr_output_fwd) = brr_output.clone_uni(k);

            let output = brr_output
                .map(k, |input| (input.inner, Expr::<Bits<U<INPUT_DIRS>>>::from(1) << input.grant_encoded).into());
            let scheduled = brr_output_fwd
                .map(k, |input| Expr::<Bits<U<INPUT_DIRS>>>::from(1) << input.inner.grant_encoded)
                .buffer(k, 0.into());

            (output, scheduled)
        },
    )
    .loop_feedback()
    .build()
}
