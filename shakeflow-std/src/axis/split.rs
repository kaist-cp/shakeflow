//! Splits the first M bits (header) of the AXI packet out of the data stream.
//! The remaining data is streamed without the first M bits.

use super::*;

/// State of split's inner FSM.
#[derive(Debug, Clone, Signal)]
struct State<WIDTH: Num, M: Num> {
    /// Buffer for the header.
    buf_header: Keep<M, Quot<M, U<8>>>,

    /// Buffer for the remaining data.
    // buf_data: Keep<WIDTH - (M % WIDTH), (WIDTH - (M % WIDTH)) / 8>
    buf_data: Keep<Diff<WIDTH, Mod<M, WIDTH>>, Quot<Diff<WIDTH, Mod<M, WIDTH>>, U<8>>>,

    /// Index to `buf_header` (in bytes).
    idx_header: Bits<Log2<M>>,

    /// Is the header prepared?
    header_prepared: bool,

    /// Is the header transfered?
    header_transfered: bool,

    /// Is the last data prepared?
    last_prepared: bool,

    /// Is the last data transfered?
    last_transfered: bool,
}

impl<WIDTH: Num, M: Num> Default for State<WIDTH, M> {
    fn default() -> Self {
        Self {
            buf_header: Keep { tdata: Bits::default(), tkeep: Bits::default() },
            buf_data: Keep { tdata: Bits::default(), tkeep: Bits::default() },
            idx_header: Bits::default(),
            header_prepared: false,
            header_transfered: false,
            last_prepared: false,
            last_transfered: false,
        }
    }
}

fn split_m<const P: Protocol, WIDTH: Num, M: Num>() -> Module<
    AxisChannel<Keep<WIDTH, Quot<WIDTH, U<8>>>, P>,
    (VrChannel<Keep<M, Quot<M, U<8>>>, P>, AxisChannel<Keep<WIDTH, Quot<WIDTH, U<8>>>, P>),
> {
    composite::<
        AxisChannel<Keep<WIDTH, Quot<WIDTH, U<8>>>, P>,
        (VrChannel<Keep<M, Quot<M, U<8>>>, P>, AxisChannel<Keep<WIDTH, Quot<WIDTH, U<8>>>, P>),
        _,
    >("split", Some("in"), Some("out"), |input, k| {
        input.fsm(
            k,
            None,
            State::<WIDTH, M>::default().into(),
            |ingress_fwd: Expr<AxisValid<AxisValue<Keep<WIDTH, Quot<WIDTH, U<8>>>>>>,
             egress_bwd: Expr<(Ready, AxisReady)>,
             s| {
                let ingress_ready = !s.header_prepared | (!s.last_prepared & egress_bwd.1.tready);
                let ingress_transfer = ingress_fwd.tvalid & ingress_ready;
                let egress_header_valid = s.header_prepared & !s.header_transfered;
                let egress_header_transfer = egress_header_valid & egress_bwd.0.ready;
                let egress_data_valid =
                    s.header_prepared & s.last_prepared.cond(!s.last_transfered, ingress_fwd.tvalid);
                let egress_data_transfer = egress_data_valid & egress_bwd.1.tready;

                let egress_header = Expr::<Valid<_>>::new(egress_header_valid, s.buf_header);
                let append_tdata = s.last_prepared.cond(0.into(), ingress_fwd.inner.payload.tdata.clip::<Mod<M, WIDTH>>(0.into()));
                let append_tkeep = s.last_prepared.cond(0.into(), ingress_fwd.inner.payload.tkeep.clip::<Quot<Mod<M, WIDTH>, U<8>>>(0.into()));
                let egress_data_tdata = s.buf_data.tdata.append(append_tdata).resize();
                let egress_data_tkeep = s.buf_data.tkeep.append(append_tkeep).resize();
                let buf_data_tdata = ingress_fwd.inner.payload.tdata.clip::<Diff<WIDTH, Mod<M, WIDTH>>>((M::WIDTH % WIDTH::WIDTH).into());
                let buf_data_tkeep = ingress_fwd.inner.payload.tkeep.clip::<Quot<Diff<WIDTH, Mod<M, WIDTH>>, U<8>>>(((M::WIDTH % WIDTH::WIDTH)/8).into());

                // egress_data_tlast = last_prepared OR (TLAST is true AND ingress is valid AND the TKEEP of the remaining packet is all zero)
                let is_last_packet = (ingress_fwd.inner.tlast & ingress_fwd.tvalid) & buf_data_tkeep.is_eq(0.into());
                let egress_data_tlast = s.last_prepared | is_last_packet;
                let egress_data = AxisValidProj {
                    tvalid: egress_data_valid,
                    inner: AxisValueProj {
                        payload: KeepProj { tdata: egress_data_tdata, tkeep: egress_data_tkeep }.into(),
                        tlast: egress_data_tlast,
                    }
                    .into(),
                }
                .into();
                let ingress = AxisReadyProj { tready: ingress_ready }.into();

                // Updates the state with the ingress packet.
                let s = select! {
                    // Populates the header.
                    ingress_transfer & !s.header_prepared => {
                        // prepared = (s.idx_header + WIDTH / 8) >= M / 8;
                        let prepared = (s.idx_header.resize::<Log2::<Sum::<M, WIDTH>>>() + Expr::<Bits<Log2::<Sum::<M, WIDTH>>>>::from(WIDTH::WIDTH / 8)).is_ge((M::WIDTH / 8).into());
                        let header_tdata = s.buf_header.tdata.set_range((s.idx_header * Expr::<Bits<Log2<M>>>::from(8)).resize(), ingress_fwd.inner.payload.tdata);
                        let header_tkeep = s.buf_header.tkeep.set_range(s.idx_header.resize(), ingress_fwd.inner.payload.tkeep);

                        let data_tdata = ingress_fwd.inner.payload.tdata.clip((M::WIDTH % WIDTH::WIDTH).into());
                        let data_tkeep = ingress_fwd.inner.payload.tkeep.clip(((M::WIDTH % WIDTH::WIDTH)/8).into());
                        let next_idx = prepared.cond((M::WIDTH / 8).into(), s.idx_header + (WIDTH::WIDTH / 8).into());

                        s.set_buf_header(KeepProj { tdata: header_tdata, tkeep: header_tkeep }.into())
                            .set_buf_data(KeepProj { tdata: data_tdata, tkeep: data_tkeep }.into())
                            .set_header_prepared(prepared)
                            .set_idx_header(next_idx.resize())
                    },
                    // Splices to the data.
                    ingress_transfer & !s.last_prepared & egress_bwd.1.tready => {
                        s.set_buf_data(KeepProj{tdata: buf_data_tdata, tkeep: buf_data_tkeep}.into())
                    },
                    default => s,
                };

                // Updates the state with the egress header.
                let header_transfered_next = s.header_transfered | egress_header_transfer;

                // Updates the state if egress TLAST should be true.
                let last_prepared_next = s.last_prepared | (ingress_fwd.inner.tlast & ingress_fwd.tvalid);

                // Updates the state with the egress data.
                let last_transfered_next = s.last_transfered | (egress_data_tlast & egress_data_transfer);

                let s_next = StateProj {
                    buf_header: s.buf_header,
                    idx_header: s.idx_header,
                    buf_data: s.buf_data,
                    header_prepared: s.header_prepared,
                    header_transfered: header_transfered_next,
                    last_prepared: last_prepared_next,
                    last_transfered: last_transfered_next
                }.into();

                // Cleans up the state if the current packet is processed completely.
                let s = (header_transfered_next & last_transfered_next).cond(State::default().into(), s_next);

                ((egress_header, egress_data).into(), ingress, s)
            },
        )
    })
    .build()
}

impl<const P: Protocol, WIDTH: Num> AxisChannel<Keep<WIDTH, Quot<WIDTH, U<8>>>, P> {
    /// Split the first M bytes out of the ingress and egress it to a seperate channel.
    /// Output the remaining bytes to another channel.
    pub fn split<M: Num>(
        self, k: &mut CompositeModuleContext,
    ) -> (VrChannel<Keep<M, Quot<M, U<8>>>, P>, AxisChannel<Keep<WIDTH, Quot<WIDTH, U<8>>>, P>) {
        self.comb_inline(k, split_m())
    }
}
