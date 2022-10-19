use shakeflow::*;
use shakeflow_std::*;

use super::bsg_channel_tunnel_out::IC as EC;

pub type IC<Width: Num, const N: usize> = VrChannel<(Bits<Width>, Bits<Log2<U<N>>>)>;

pub fn m<Width: Num, const N: usize, const REMOTE_CREDITS: usize>() -> Module<IC<Width, N>, EC<Width, N, REMOTE_CREDITS>>
where
    [(); 1 << N]:,
    [(); N + 1]:,
{
    composite::<IC<Width, N>, EC<Width, N, REMOTE_CREDITS>, _>(
        "bsg_channel_tunnel_in",
        Some("i"),
        Some("o"),
        |input, k| {
            let (input, tag) = input.clone_uni(k);
            let input = input.map(k, |input| input.0);
            let tag = tag.map(k, |input| input.inner.1.resize());

            let mut demuxed =
                (input, tag)
                    .comb_inline(
                        k,
                        super::bsg_1_to_n_tagged_fifo::m::<
                            Bits<Width>,
                            { N + 1 },
                            REMOTE_CREDITS,
                            { 1 << N },
                            false,
                            false,
                        >(),
                    )
                    .into_iter();
            let credit = demuxed.next_back().unwrap().into_uni(k, false);
            let outgoing: [VrChannel<Bits<Width>>; N] = demuxed.collect::<Vec<_>>().try_into().unwrap();

            // XXX: We assume that input has bit-representable signal?
            let credit_local_return = credit.map_inner(k, |input| {
                input.resize::<Prod<Log2<U<REMOTE_CREDITS>>, U<N>>>().chunk::<Log2<U<REMOTE_CREDITS>>>().resize()
            });

            let (outgoing, sent) = outgoing.array_map(k, "sent", |ch, k| ch.fire(k)).unzip();

            let credit_remote_return = sent
                .array_map(k, "credit_remote_return", |ch, k| ch.counter(k).map(k, |input| input.0))
                .concat(k)
                .into_deq(k)
                .fsm_map(k, ().into(), |input, state| (input, state, true.into()));

            EC { data: outgoing, credit_local_return, credit_remote_return }
        },
    )
    .build()
}
