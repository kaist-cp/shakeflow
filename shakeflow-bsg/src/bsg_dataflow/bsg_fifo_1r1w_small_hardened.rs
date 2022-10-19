use shakeflow::*;
use shakeflow_std::*;

pub type C<V: Signal> = VrChannel<V>;

pub fn m<V: Signal, const SLOTS: usize>() -> Module<C<V>, C<V>> {
    composite::<C<V>, C<V>, _>("bsg_fifo_1r1w_small_hardened", Some("i"), Some("o"), |input, k| {
        input.fifo_1r1w(k, |input, k| {
            let write = input.clone().map(k, |input| input.write);
            let read = input.map(k, |input| input.read_n);

            write.zip(k, read).fsm_map::<(Array<V, U<SLOTS>>, V), V, _>(k, Some("mem"), Expr::x(), |input, state| {
                let (write, read) = *input;
                let (state, output) = *state;
                let state_next = (
                    state.set(write.inner.0, write.valid.cond(write.inner.1, state[write.inner.0])),
                    write.inner.0.is_eq(read).cond(write.inner.1, state[read]),
                )
                    .into();
                (output, state_next)
            })
        })
    })
    .build()
}
