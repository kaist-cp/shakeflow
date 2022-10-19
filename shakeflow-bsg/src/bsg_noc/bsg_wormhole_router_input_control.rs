use shakeflow::*;
use shakeflow_std::*;

#[derive(Debug, Clone, Signal)]
pub struct I<const OUTPUT_DIRS: usize, const PAYLOAD_LEN_BITS: usize> {
    decoded_dest: Bits<U<OUTPUT_DIRS>>,
    payload_len: Bits<U<PAYLOAD_LEN_BITS>>,
}

#[derive(Debug, Clone, Signal)]
pub struct E<const OUTPUT_DIRS: usize> {
    reqs: Bits<U<OUTPUT_DIRS>>,
    release: bool,
    detected_header: bool,
}

pub type IC<const OUTPUT_DIRS: usize, const PAYLOAD_LEN_BITS: usize> =
    UniChannel<(Valid<I<OUTPUT_DIRS, PAYLOAD_LEN_BITS>>, bool)>;
pub type EC<const OUTPUT_DIRS: usize> = UniChannel<E<OUTPUT_DIRS>>;

pub fn m<const OUTPUT_DIRS: usize, const PAYLOAD_LEN_BITS: usize>(
) -> Module<IC<OUTPUT_DIRS, PAYLOAD_LEN_BITS>, EC<OUTPUT_DIRS>> {
    composite::<IC<OUTPUT_DIRS, PAYLOAD_LEN_BITS>, EC<OUTPUT_DIRS>, _>(
        "bsg_wormhole_router_input_control",
        Some("i"),
        Some("o"),
        |input, k| {
            input.fsm_map::<Bits<U<PAYLOAD_LEN_BITS>>, E<OUTPUT_DIRS>, _>(
                k,
                None,
                0.into(),
                |input, payload_count| {
                    let (input, sent) = *input;
                    let counter_expired = payload_count.is_eq(0.into());
                    let fifo_has_hdr = counter_expired & input.valid;

                    let payload_count_next = select! {
                        sent & counter_expired => input.inner.payload_len,
                        sent & !counter_expired => payload_count - 1.into(),
                        default => payload_count,
                    };

                    let output = EProj {
                        reqs: fifo_has_hdr.cond(input.inner.decoded_dest, 0.into()),
                        release: counter_expired,
                        detected_header: fifo_has_hdr,
                    }
                    .into();

                    (output, payload_count_next)
                }
            )
        },
    )
    // .loop_feedback()
    .build()
}
