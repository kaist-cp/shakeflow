use shakeflow::*;
use shakeflow_std::*;

pub type C<V: Signal> = [VrChannel<V>; 2];

pub fn m<V: Signal>() -> Module<C<V>, C<V>> {
    composite::<C<V>, C<V>, _>("bsg_round_robin_2_to_2", Some("i"), Some("o"), |input, k| {
        let (input, fires) = input.array_map(k, "fires", |ch, k| ch.fire(k)).unzip();
        let fire = fires.concat(k).map(k, |f| f[0] ^ f[1]);

        let select = fire.fsm_map::<bool, Array<Bits<Log2<U<2>>>, U<2>>, _>(k, None, false.into(), |input, state| {
            let output: Expr<Bits<U<2>>> = state.cond(0b10.into(), 0b01.into());
            let output = output.map(|i| i.repr().resize());
            let state_next = state ^ input;
            (output, state_next)
        });

        input.permute(k, (select.clone(), select))
    })
    .build()
}
