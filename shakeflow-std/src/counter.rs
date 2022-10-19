//! Counter modules.

use crate::num::*;
use crate::*;

impl UniChannel<bool> {
    /// Circular pointer.
    ///
    /// It returns the pair of current counter value and next counter value.
    pub fn counter<N: Num>(self, k: &mut CompositeModuleContext) -> UniChannel<(Bits<Log2<N>>, Bits<Log2<N>>)> {
        self.fsm_map::<Bits<Log2<N>>, (Bits<Log2<N>>, Bits<Log2<N>>), _>(k, None, 0.into(), |input, state| {
            let state_next = state + input.repr().resize();
            let state_next = state_next.is_ge(N::WIDTH.into()).cond(0.into(), state_next).resize();
            ((state, state_next).into(), state_next)
        })
    }
}

impl<V: Signal> VrChannel<V> {
    /// Counts the number of transfer.
    pub fn counter_transfer<N: Num>(self, k: &mut CompositeModuleContext) -> (VrChannel<V>, UniChannel<Bits<Log2<N>>>) {
        let (this, fire) = self.fire(k);
        let counter = fire.counter::<N>(k).map(k, |input| input.0);
        (this, counter)
    }
}

/// Counter up and down extension.
pub trait CounterUpDownExt {
    /// Counter up and down function.
    fn counter_up_down<N: Num>(self, k: &mut CompositeModuleContext) -> UniChannel<Bits<N>>;
}

impl CounterUpDownExt for (UniChannel<bool>, UniChannel<bool>) {
    fn counter_up_down<N: Num>(self, k: &mut CompositeModuleContext) -> UniChannel<Bits<N>> {
        let (up, down) = self;
        up.zip(k, down).fsm_map::<Bits<N>, Bits<N>, _>(k, None, 0.into(), |input, count| {
            let (up, down) = *input;
            let count_next = (count - down.repr().resize() + up.repr().resize()).resize();
            (count, count_next)
        })
    }
}

/// Counter clear up one hot extension.
pub trait CounterClearUpOneHotExt {
    /// Counter clear up one hot function.
    fn counter_clear_up_one_hot<N: Num>(self, k: &mut CompositeModuleContext) -> UniChannel<Bits<N>>;
}

impl CounterClearUpOneHotExt for (UniChannel<bool>, UniChannel<bool>) {
    fn counter_clear_up_one_hot<N: Num>(self, k: &mut CompositeModuleContext) -> UniChannel<Bits<N>> {
        let (clear, up) = self;
        clear.zip(k, up).fsm_map::<Bits<N>, Bits<N>, _>(k, None, 0.into(), |input, count| {
            let (clear, up) = *input;
            let count_next = clear.cond(1.into(), up.cond(count << 1, count));
            (count, count_next)
        })
    }
}
