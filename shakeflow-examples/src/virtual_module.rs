use shakeflow::*;
use shakeflow_std::*;

#[allow(clippy::type_complexity)]
fn m_split() -> Module<(VrChannel<Bits<U<8>>>, VrChannel<Bits<U<8>>>), (VrChannel<Bits<U<64>>>, VrChannel<Bits<U<8>>>)>
{
    composite::<(VrChannel<Bits<U<8>>>, VrChannel<Bits<U<8>>>), (VrChannel<Bits<U<64>>>, VrChannel<Bits<U<8>>>), _>(
        "split",
        Some("i"),
        Some("o"),
        |i, k| (i.0.map(k, |v| v.resize()), i.1),
    )
    .build()
}

pub fn read_write_test_m() -> Module<VrChannel<Bits<U<8>>>, VrChannel<Bits<U<8>>>> {
    composite::<VrChannel<Bits<U<8>>>, VrChannel<Bits<U<8>>>, _>("rw", Some("i"), Some("o"), |i, k| {
        let (r, w) = k.register(None, m_split()).split();
        i.comb_inline(k, r).map(k, |i| i.resize()).comb_inline(k, w).map(k, |i| i.resize())
    })
    .build()
}

#[allow(clippy::type_complexity)]
pub fn read_write_array_test_m() -> Module<[VrChannel<Bits<U<8>>>; 3], [VrChannel<Bits<U<8>>>; 3]> {
    composite::<VrChannel<Bits<U<8>>>, VrChannel<Bits<U<8>>>, _>("rw_array", Some("i"), Some("o"), |i, k| {
        let (r, w) = k.register(None, m_split()).split();
        i.comb_inline(k, r).map(k, |i| i.resize()).comb_inline(k, w).map(k, |i| i.resize())
    })
    .build_array()
}

pub fn feedback_test_m() -> Module<UniChannel<bool>, UniChannel<bool>> {
    composite::<UniChannel<bool>, UniChannel<bool>, _>("feedback_inline", Some("i"), Some("o"), |i, k| {
        let (source, sink) = k.feedback::<UniChannel<bool>>();

        let out = source.map(k, |b| !b);

        i.map(k, |b| !b).comb_inline(k, sink);

        out
    })
    .build()
}

pub fn read_write_inline_test_m() -> Module<VrChannel<Bits<U<8>>>, VrChannel<Bits<U<8>>>> {
    composite::<VrChannel<Bits<U<8>>>, VrChannel<Bits<U<8>>>, _>("rw_inline", Some("i"), Some("o"), |i, k| {
        let (r, w) = k.register_inline(m_split()).split();
        i.comb_inline(k, r).map(k, |i| i.resize()).comb_inline(k, w).map(k, |i| i.resize())
    })
    .build()
}
