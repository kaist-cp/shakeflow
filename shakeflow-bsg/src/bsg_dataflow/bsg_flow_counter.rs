use shakeflow::*;
use shakeflow_std::*;

pub type IC<V: Signal> = VrChannel<V>;
pub type EC<V: Signal, const ELS: usize> = (VrChannel<V>, UniChannel<Bits<Log2<U<ELS>>>>);

pub fn m<V: Signal, const ELS: usize, const COUNT_FREE: bool>(
    module: Module<IC<V>, IC<V>>,
) -> Module<IC<V>, EC<V, ELS>> {
    composite::<IC<V>, EC<V, ELS>, _>("bsg_flow_counter", Some("i"), Some("o"), |input, k| {
        let (input, input_fire) = input.fire(k);
        let output = input.comb_inline(k, module);
        let (output, output_fire) = output.fire(k);

        let remaining = (input_fire, output_fire).counter_up_down::<U<ELS>>(k);
        let counter = if COUNT_FREE {
            remaining.map(k, |input| Expr::<Bits<Log2<U<ELS>>>>::from(ELS) - input.resize())
        } else {
            remaining.map(k, |input| input.resize())
        };

        (output, counter)
    })
    .build()
}
