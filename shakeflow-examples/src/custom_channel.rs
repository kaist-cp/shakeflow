use shakeflow::*;
use shakeflow_std::*;

#[derive(Debug, Clone, Signal)]
pub struct CustomValue {
    bwd: bool,
    a: u8,
}

channel! {CustomMonoMorphicChannel, CustomValue, CustomValue}

channel! {CustomUniChannel<V: Signal>, V, ()}

channel! {CustomReverseUniChannel<V: Signal>, (), V}

channel! {CustomChannel<V: Signal>, V, CustomValue}

channel! {CustomBiChannel<V: Signal>, V, V}

channel! {CustomReverseChannel<V: Signal>, CustomValue, V}

channel! {CustomValidChannel<V: Signal>, Valid<V>, CustomValue}

channel! {CustomReverseValidChannel<V: Signal>, CustomValue, Valid<V>}

channel! {CustomBidirValidChannel<V: Signal>, Valid<V>, Valid<V>}

// multiple generic not supported for now (need some extra work since phantomdata can only have 1
// generic arguement)
// channel!{CustomMultiGenericChannel<V: Signal, W: Signal>, V, W,}
