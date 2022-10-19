use itertools::*;
use shakeflow::*;
use shakeflow_std::axis::*;
use shakeflow_std::{FsmExt, UniChannel, Valid, ValidExt};

#[derive(Debug, Interface)]
pub struct I<const DATA_WIDTH: usize, const KEEP_WIDTH: usize> {
    s_axis: UniChannel<AxisValid<AxisValue<Keep<U<DATA_WIDTH>, U<KEEP_WIDTH>>>>>,
    hash_key: UniChannel<Bits<U<{ 40 * 8 }>>>,
}
pub type O = UniChannel<Valid<Output>>;

// const CYCLE_COUNT: usize = (38 + KEEP_WIDTH - 1) / KEEP_WIDTH;

// TODO: Parametrize
const PTR_WIDTH: usize = 1;

/// Packet's protocol.
#[derive(Debug, Clone, Signal)]
struct Protocol {
    ipv4: bool,
    tcp: bool,
    udp: bool,
    typ: Bits<U<4>>,
}

/// Hash result.
#[derive(Debug, Clone, Signal)]
struct HashIpv4 {
    hash_ip: Bits<U<32>>,
    hash_port: Bits<U<32>>,
}

/// State for inner Fsm component.
#[derive(Debug, Clone, Signal)]
struct State {
    active: bool,
    ptr: Bits<U<PTR_WIDTH>>,
    eth_type: Bits<U<16>>,
    ihl: Bits<U<4>>,
    data: Bits<U<{ 36 * 8 }>>,
    #[member(name = "")]
    protocol: Protocol,
}

/// Output of the module.
#[derive(Debug, Clone, Signal)]
pub struct Output {
    #[member(name = "")]
    hash: Bits<U<32>>,
    #[member(name = "type")]
    typ: Bits<U<4>>,
}

/// Computes toeplitz hash.
fn hash_toep<'id>(
    data: Expr<'id, Bits<U<{ 36 * 8 }>>>, len: usize, key: Expr<'id, Bits<U<{ 40 * 8 }>>>,
) -> Expr<'id, Bits<U<32>>> {
    iproduct!(0..len, 0..8).fold(0.into(), |hash_toep, (i, j)| {
        hash_toep ^ data[i * 8 + (7 - j)].cond(key.clip_const::<U<32>>(40 * 8 - 32 - i * 8 - j), 0.into())
    })
}

pub fn m<const DATA_WIDTH: usize, const KEEP_WIDTH: usize>(module_name: &str) -> Module<I<DATA_WIDTH, KEEP_WIDTH>, O> {
    composite::<I<DATA_WIDTH, KEEP_WIDTH>, _, _>(module_name, None, Some("m_axis_hash"), |value, k| {
        value
            .s_axis
            .zip(k, value.hash_key)
            // Deserializes packet and computes hash.
            .fsm_map::<State, Valid<(HashIpv4, Protocol)>, _>(
                k,
                None,
                StateProj {
                    active: true.into(),
                    ptr: [false; PTR_WIDTH].into(),
                    eth_type: Expr::x(),
                    ihl: Expr::x(),
                    data: Expr::x(),
                    protocol: Expr::x(),
                }
                .into(),
                |input, state| {
                    let (axis_fwd, hash_key) = *input;
                    let data = axis_fwd.inner;
                    let tvalid = axis_fwd.tvalid;
                    let payload = data.payload;
                    let tdata = payload.tdata;
                    let tlast = data.tlast;

                    let active = state.active;
                    let ptr = state.ptr;
                    let eth_type = state.eth_type;
                    let ihl = state.ihl;
                    let data = state.data.chunk::<U<8>>();
                    let protocol = state.protocol;

                    let is_active = tvalid & active;

                    // Resets protocol if ptr is zero
                    let is_protocol_rst = is_active & ptr.is_eq(0.into());
                    let ipv4 = protocol.ipv4 & !is_protocol_rst;
                    let tcp = protocol.tcp & !is_protocol_rst;
                    let udp = protocol.udp & !is_protocol_rst;

                    // Captures ethernet type
                    let eth_type = (is_active & ptr.is_eq((13 / KEEP_WIDTH).into()))
                        .cond(tdata.clip_const::<U<8>>((13 % KEEP_WIDTH) * 8), eth_type.clip_const::<U<8>>(0))
                        .append(
                            (is_active & ptr.is_eq((12 / KEEP_WIDTH).into()))
                                .cond(tdata.clip_const::<U<8>>((12 % KEEP_WIDTH) * 8), eth_type.clip_const::<U<8>>(8)),
                        )
                        .resize();

                    // Checks the packet is IPv4
                    let ipv4 = ipv4
                        | ((is_active & ptr.is_eq((13 / KEEP_WIDTH).into())) & eth_type.is_eq(0x800.into()));

                    // Updates type, valid, active
                    let typ = is_active.cond(
                        ptr.is_eq((13 / KEEP_WIDTH).into()).cond(
                            eth_type
                                .is_eq(0x800.into())
                                .cond(protocol.typ, [false; 4].into()),
                            protocol.typ,
                        ),
                        protocol.typ,
                    );
                    let valid = is_active & ptr.is_eq((13 / KEEP_WIDTH).into()) & !eth_type.is_eq(0x800.into());
                    let active = active & !valid;

                    // Captures IHL
                    let ihl = (is_active & ptr.is_eq((14 / KEEP_WIDTH).into()))
                        .cond(tdata.clip_const::<U<4>>((14 % KEEP_WIDTH) * 8), ihl);

                    let is_ipv4 = is_active & ipv4;

                    // Captures protocol
                    let tcp = tcp
                        | (is_ipv4
                            & ptr.is_eq((23 / KEEP_WIDTH).into())
                            & tdata.clip_const::<U<8>>((23 % KEEP_WIDTH) * 8).is_eq(0x6.into())
                            & ihl.is_eq(5.into()));
                    let udp = udp
                        | (is_ipv4
                            & ptr.is_eq((23 / KEEP_WIDTH).into())
                            & tdata.clip_const::<U<8>>((23 % KEEP_WIDTH) * 8).is_eq(0x11.into())
                            & ihl.is_eq(5.into()));

                    // Captures source IP and dest IP
                    let data = (26..=33)
                        .fold(data, |data, i| {
                            if_then_set! { data, is_ipv4 & ptr.is_eq((i / KEEP_WIDTH).into()), Expr::from(i - 26), tdata.clip_const::<U<8>>((i % KEEP_WIDTH) * 8) }
                        });

                    let is_port_avail = is_ipv4 & (tcp | udp);

                    // Updates type, valid and active
                    let typ = (is_ipv4 & ptr.is_eq((33 / KEEP_WIDTH).into()) & !is_port_avail)
                        .cond([true, false, false, false].into(), typ);
                    let valid =
                        valid | (is_ipv4 & ptr.is_eq((33 / KEEP_WIDTH).into()) & !is_port_avail);
                    let active = active & !(is_ipv4 & ptr.is_eq((33 / KEEP_WIDTH).into()) & !is_port_avail);

                    // Captures source port and dest port
                    let data = (34..=37)
                        .fold(data, |data, i| {
                            if_then_set! { data, is_port_avail & ptr.is_eq((i / KEEP_WIDTH).into()), Expr::from(i - 26), tdata.clip_const::<U<8>>((i % KEEP_WIDTH) * 8) }
                        })
                        .repr();

                    let typ = (is_port_avail & ptr.is_eq((37 / KEEP_WIDTH).into())).cond(
                        Expr::from([true, false])
                            .append(tcp.into())
                            .append(udp.into())
                            .resize(),
                        typ,
                    );
                    let valid = valid | (is_port_avail & ptr.is_eq((37 / KEEP_WIDTH).into()));
                    let active = active & !(is_port_avail & ptr.is_eq((37 / KEEP_WIDTH).into()));

                    // Updates type and valid
                    let is_active_last = tvalid & tlast & active;
                    let typ = is_active_last.cond([false; 4].into(), typ);
                    let valid = valid | is_active_last;

                    // Updates ptr and active
                    let ptr = (tvalid & tlast).cond(
                        0.into(),
                        is_active.cond((ptr + 1.into()).resize::<U<PTR_WIDTH>>(), ptr),
                    );
                    let active = active | (tvalid & tlast);

                    // Calculates hash results
                    let hash_ip = hash_toep(data, 8, hash_key);
                    let hash_port = hash_toep(data >> 64_usize, 4, hash_key.resize() << 64_usize);

                    let protocol = ProtocolProj { ipv4, tcp, udp, typ }.into();
                    (
                        Expr::<Valid<_>>::new(
                            valid,
                            (HashIpv4Proj { hash_ip, hash_port }.into(), protocol).into(),
                        ),
                        StateProj {
                            active,
                            ptr,
                            eth_type,
                            ihl,
                            data,
                            protocol,
                        }
                        .into(),
                    )
                },
            )
            .buffer(k, Expr::invalid())
            // Post-processing
            .map_inner(k, |inner| {
                let (hash, protocol) = *inner;
                OutputProj {
                    hash: (!protocol.ipv4).cond(
                        [false; 32].into(),
                        (protocol.tcp | protocol.udp).cond(hash.hash_ip ^ hash.hash_port, hash.hash_ip),
                    ),
                    typ: protocol.typ,
                }
                .into()
            })
            .buffer(k, Expr::invalid())
    })
    .build()
}
