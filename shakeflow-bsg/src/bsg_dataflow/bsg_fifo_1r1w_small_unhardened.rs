use shakeflow::*;
use shakeflow_std::*;

pub type C<V: Signal> = VrChannel<V>;

pub fn m<V: Signal, const SLOTS: usize>() -> Module<C<V>, C<V>> {
    composite::<C<V>, C<V>, _>("bsg_fifo_1r1w_small_unhardened", Some("i"), Some("o"), |input, k| {
        input.fifo_1r1w(k, |input, k| {
            let write = input.clone().map(k, |input| input.write);
            let read = input.map(k, |input| input.read);

            write.zip(k, read).fsm_map::<Array<V, U<SLOTS>>, V, _>(k, Some("mem"), Expr::x(), |input, state| {
                let (write, read) = *input;
                let output = state[read];
                let state_next = state.set(write.inner.0, write.valid.cond(write.inner.1, state[write.inner.0]));
                (output, state_next)
            })
        })
    })
    .build()
}
