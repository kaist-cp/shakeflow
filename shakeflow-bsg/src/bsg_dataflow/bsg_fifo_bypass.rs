use shakeflow::*;
use shakeflow_std::*;

pub type C<V: Signal> = VrChannel<V>;

pub fn m<V: Signal>(fifo: Module<C<V>, C<V>>) -> Module<C<V>, C<V>> {
    composite::<C<V>, C<V>, _>("bsg_fifo_bypass", Some("i"), Some("o"), |input, k| input.fifo_bypass(k, fifo)).build()
}
