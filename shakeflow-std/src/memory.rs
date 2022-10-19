//! Memory

use crate::*;

/// Memory Ingress Channel
pub type MemI<KEY, ENTRY> = (VrChannel<KEY>, VrChannel<(KEY, ENTRY)>);

/// Memory Engress Channel
pub type MemO<KEY, ENTRY> = (VrChannel<(KEY, ENTRY)>, VrChannel<(KEY, ENTRY)>);

// impl<KEY: Signal, ENTRY: Signal> MemI<KEY, ENTRY> {
//     /// Creates a new Memory Ingress Channel
//     pub fn new(read: VrChannel<KEY>, write: VrChannel<(KEY, ENTRY)>) -> Self { Self { read, write } }
// }

/// Key type given size of the memory
type KeyType<const MEM_SIZE: usize> = Bits<Log2<U<MEM_SIZE>>>;

/// Memory with Read, Write Valid Signal UniChannels
///
///# NOTE
///
/// When valid read and valid write request arrive at the same time, both will be consumed
pub fn m<ENTRY: Signal, const PIPELINE: usize, const NUM_ENTRIES: usize>(
    init_entry: Expr<'static, ENTRY>,
) -> Module<MemI<KeyType<NUM_ENTRIES>, ENTRY>, MemO<KeyType<NUM_ENTRIES>, ENTRY>> {
    // Memory size should be power of two
    assert!(NUM_ENTRIES.is_power_of_two());

    // TODO
    // Add pipelining for BRAM or URAM access (for hash table, it seems we do not need this right now since each table is 81(entry size) * 64 (number of entries) = 5,184 bits only)
    composite::<MemI<KeyType<NUM_ENTRIES>, ENTRY>, MemO<KeyType<NUM_ENTRIES>, ENTRY>, _>(
        "memory",
        Some("in"),
        Some("out"),
        |input, k| {
            let (selector, input): (UniChannel<Bits<U<2>>>, VrChannel<(KeyType<NUM_ENTRIES>, ENTRY)>) =
                [input.0.map(k, |input| (input, Expr::x()).into()), input.1].mux(k);

            let [read_out, write_out] = input
                .zip_uni(k, selector)
                .fsm_map(k, Some("memory"), init_entry.repeat(), |input, state| {
                    let (input, selector) = *input;
                    let (key, value) = *input;

                    let read_entry = state[key];

                    let state_next = select! {
                        selector.is_eq(1.into()) => state,
                        selector.is_eq(2.into()) => state.set(key, value),
                        default => state,
                    };

                    let selector: Expr<Bits<U<1>>> = select! {
                        selector.is_eq(1.into()) => 0.into(),
                        selector.is_eq(2.into()) => 1.into(),
                        default => 0.into(),
                    };

                    let output: Expr<_> = (key, read_entry).into();
                    ((output, selector).into(), state_next)
                })
                .demux(k);

            (read_out, write_out)
        },
    )
    .build()
}
