use shakeflow::*;
use shakeflow_std::*;

use super::bsg_channel_tunnel_out::IC as TunnelOutIC;

#[derive(Debug, Interface)]
pub struct C<Width: Num, const N: usize> {
    muxed: VrChannel<(Bits<Width>, Bits<Log2<U<N>>>)>,
    demuxed: [VrChannel<Bits<Width>>; N],
}

pub fn m<Width: Num, const N: usize, const REMOTE_CREDITS: usize>() -> Module<C<Width, N>, C<Width, N>>
where
    [(); 1 << N]:,
    [(); N + 1]:,
{
    composite::<C<Width, N>, C<Width, N>, _>("bsg_channel_tunnel", Some("i"), Some("o"), |input, k| {
        let C { muxed: muxed_input, demuxed: demuxed_input } = input;

        let tunnel_in_output =
            muxed_input.comb_inline(k, super::bsg_channel_tunnel_in::m::<Width, N, REMOTE_CREDITS>());

        let tunnel_out_output = TunnelOutIC {
            data: demuxed_input,
            credit_local_return: tunnel_in_output.credit_local_return,
            credit_remote_return: tunnel_in_output.credit_remote_return,
        }
        .comb_inline(k, super::bsg_channel_tunnel_out::m::<Width, N, REMOTE_CREDITS>());

        C { muxed: tunnel_out_output, demuxed: tunnel_in_output.data }
    })
    .build()
}
