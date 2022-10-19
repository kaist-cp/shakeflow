//! Splice a packet to units of WIDTH bits, and write them to egress one at a time.

use super::*;

fn chunk_m<WIDTH: Num, V: Signal + Serialize>() -> Module<AxisVrChannel<V>, AxisChannel<Keep<WIDTH, Quot<WIDTH, U<8>>>>>
where [(); V::WIDTH + 1]: {
    composite::<AxisVrChannel<V>, AxisChannel<Keep<WIDTH, Quot<WIDTH, U<8>>>>, _>(
        "chunk",
        Some("in"),
        Some("out"),
        |input, k| {
            input
                .into_vr(k)
                .fsm_egress(
                    k,
                    None,
                    Bits::<Diff<Log2<U<{ V::WIDTH }>>, U<2>>>::default().into(),
                    |packet, byte_index| {
                        let next_index = (byte_index * Expr::<Bits<U<4>>>::from(8))
                            .resize::<Sum<Log2<U<{ V::WIDTH }>>, U<2>>>()
                            + WIDTH::WIDTH.into();
                        let tdata = next_index.is_ge(V::WIDTH.into()).cond(
                            <V>::serialize(packet)
                                .clip::<Mod<U<{ V::WIDTH }>, WIDTH>>(
                                    (byte_index * Expr::<Bits<U<4>>>::from(8)).resize(),
                                )
                                .resize(),
                            <V>::serialize(packet).clip::<WIDTH>((byte_index * Expr::<Bits<U<4>>>::from(8)).resize()),
                        );

                        // let last = (byte_index * 8 + WIDTH) >= packet::WIDTH
                        let tlast = next_index.is_ge(V::WIDTH.into());
                        let tkeep: Expr<Bits<Quot<WIDTH, U<8>>>> = tlast.cond(
                            // If this is last write, tkeep = `(1 << remaining bytes) - 1`
                            ((Expr::<Bits<Quot<WIDTH, U<8>>>>::from(1)
                                << (Expr::<Bits<Log2<U<{ V::WIDTH + 1 }>>>>::from(V::WIDTH)
                                    - (byte_index * Expr::<Bits<U<4>>>::from(8)).resize())
                                .resize::<Log2<Quot<WIDTH, U<8>>>>())
                                - 1.into())
                            .resize::<Quot<WIDTH, U<8>>>(),
                            // If this is not last write, tkeep = `~0`
                            !(Expr::<Bits<Quot<WIDTH, U<8>>>>::from(0)),
                        );
                        (
                            AxisValueProj { tlast, payload: KeepProj { tdata, tkeep }.into() }.into(),
                            (byte_index + (WIDTH::WIDTH / 8).into()).resize(),
                            tlast,
                        )
                    },
                )
                .into_axis_vr(k)
        },
    )
    .build()
}

impl<V: Signal + Serialize> AxisVrChannel<V> {
    /// Splice a packet to units of WIDTH bits, and write them to egress one at a time.
    pub fn chunk<WIDTH: Num>(self, k: &mut CompositeModuleContext) -> AxisChannel<Keep<WIDTH, Quot<WIDTH, U<8>>>>
    // Thankfully, the calling function doesn't need to assert the following constraint if it calls this with a specified `V` type.
    where [(); V::WIDTH + 1]: {
        self.comb_inline(k, chunk_m::<WIDTH, V>())
    }
}
