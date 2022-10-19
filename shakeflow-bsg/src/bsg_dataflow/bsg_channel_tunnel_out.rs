//! Takes N channels and tunnels them, with credit flow control.

use shakeflow::*;
use shakeflow_std::*;

use super::bsg_channel_tunnel_in::IC as EC;

const CREDIT_DECIMATION: usize = 1;

#[derive(Debug, Interface)]
pub struct IC<Width: Num, const N: usize, const REMOTE_CREDITS: usize> {
    pub data: [VrChannel<Bits<Width>>; N],
    pub credit_local_return: UniChannel<Valid<Array<Bits<Log2<U<REMOTE_CREDITS>>>, U<N>>>>,
    pub credit_remote_return: DeqChannel<Array<Bits<Log2<U<REMOTE_CREDITS>>>, U<N>>>,
}

pub fn m<Width: Num, const N: usize, const REMOTE_CREDITS: usize>() -> Module<IC<Width, N, REMOTE_CREDITS>, EC<Width, N>>
where [(); N + 1]: {
    composite::<IC<Width, N, REMOTE_CREDITS>, EC<Width, N>, _>("bsg_channel_tunnel_out", None, Some("o"), |input, k| {
        let IC { data, credit_local_return, credit_remote_return } = input;

        let credit_local_return =
            credit_local_return.map(k, |input| Expr::<Valid<_>>::new_arr(input.valid.repeat(), input.inner)).slice(k);

        let data = data.array_zip(credit_local_return).array_map(
            k,
            "local_credits_avail",
            |(data, credit_local_return), k| {
                let (data, fire) = data.fire(k);

                let local_credits_avail =
                    credit_local_return.zip(k, fire).fsm_map::<Bits<Log2<U<REMOTE_CREDITS>>>, bool, _>(
                        k,
                        None,
                        REMOTE_CREDITS.into(),
                        |input, count| {
                            let (credit_local_return, down) = *input;
                            let up = credit_local_return.valid.cond(credit_local_return.inner, 0.into());
                            let count_next = count - down.repr().resize() + up.resize();
                            (count.any(), count_next.resize())
                        },
                    );

                data.zip_uni(k, local_credits_avail)
                    .and_then(k, None, |input| Expr::<Valid<_>>::new(input.1, input.0))
                    .buffer(k)
            },
        );

        let credit = credit_remote_return
            .into_vr(k)
            .and_then(k, None, |input| {
                let remote_credits_avail = input.map(|data| {
                    data.clip_const::<Diff<Log2<U<REMOTE_CREDITS>>, U<CREDIT_DECIMATION>>>(CREDIT_DECIMATION).any()
                });
                Expr::<Valid<_>>::new(remote_credits_avail.any(), input.concat().resize::<Width>())
            })
            .buffer(k);

        let rr_input: [VrChannel<Bits<Width>>; N + 1] = ::std::iter::empty()
            .chain(data.into_iter())
            .chain(::std::iter::once(credit))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let (tag, data) = rr_input.comb_inline(k, super::bsg_round_robin_n_to_1::m::<Bits<Width>, { N + 1 }, true>());

        data.zip_uni(k, tag).map(k, |input| (input.0, input.1.resize()).into())
    })
    .build()
}
