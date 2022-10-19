//! FIFO with only one read or write port, using 1RW asynchronous read ram.

use shakeflow::*;
use shakeflow_std::fifo::one_read_write::*;
use shakeflow_std::*;

pub type IC<V: Signal> = UniChannel<I<V>>;
pub type EC<V: Signal> = UniChannel<E<V>>;

pub fn m<V: Signal, const ELS: usize>() -> Module<IC<V>, EC<V>> {
    composite::<IC<V>, EC<V>, _>("bsg_fifo_1rw_large", Some("i"), Some("o"), |input, k| input.fifo_1rw::<ELS>(k))
        .build()
}
