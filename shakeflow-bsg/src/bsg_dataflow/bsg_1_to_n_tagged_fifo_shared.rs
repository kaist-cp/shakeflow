use shakeflow::*;
use shakeflow_std::*;

const WIDTH_P: usize = 10;
const NUM_OUT_P: usize = 5;
const ELS_P: usize = 5;
const BUFFERING_P: usize = 1;

const TAG_WIDTH_LP: usize = clog2(NUM_OUT_P);
const PTR_WIDTH_LP: usize = clog2(ELS_P);

#[derive(Debug, Clone, Signal)]
pub struct I {
    tag: Bits<U<TAG_WIDTH_LP>>,
    data: Bits<U<WIDTH_P>>,
}

#[derive(Debug, Clone, Signal)]
pub struct O {
    data: Bits<U<WIDTH_P>>,
}

pub type IC = VrChannel<I>;
pub type OC = [VrChannel<O>; NUM_OUT_P];

#[derive(Debug, Clone, Signal)]
pub struct B1ntI {
    tag: Bits<U<TAG_WIDTH_LP>>,
}

pub type B1ntOC = [VrChannel<()>; NUM_OUT_P];

#[derive(Debug, Clone, Signal)]
pub struct FtI {
    enq: bool,
    deq: bool,
}

#[derive(Debug, Clone, Signal)]
pub struct FtO {
    wptr_r: Bits<U<PTR_WIDTH_LP>>,
    rptr_r: Bits<U<PTR_WIDTH_LP>>,
    full: bool,
    empty: bool,
}

pub type FtOC = UniChannel<FtO>;

#[derive(Debug, Interface)]
pub struct RdrArbIC {
    grants_en: UniChannel<bool>,
    reqs: UniChannel<Bits<U<NUM_OUT_P>>>,
}

#[derive(Debug, Interface)]
pub struct RdrArbOC {
    grants: UniChannel<Bits<U<NUM_OUT_P>>>,
    sel_one_hot: UniChannel<Bits<U<NUM_OUT_P>>>,
    tag: VrChannel<Bits<U<TAG_WIDTH_LP>>>,
}

#[derive(Debug, Clone, Signal)]
pub struct BcudbI {
    up: bool,
    down: bool,
}

pub type BcudbOC = UniChannel<Bits<U<BUFFERING_P>>>;

#[derive(Debug, Clone, Signal)]
pub struct BigRamI {
    data: Bits<U<WIDTH_P>>,
    v: bool,
    w: bool,
}

pub type BigRamOC = UniChannel<Bits<U<WIDTH_P>>>;

#[derive(Debug, Clone, Signal)]
pub struct Feedback {
    yumi: Bits<U<NUM_OUT_P>>,
}

pub type FeedbackC = UniChannel<Feedback>;

impl_custom_inst! {VrChannel<B1ntI>, B1ntOC, bsg_1_to_n_tagged, <num_out_p>, true,}
impl_custom_inst! {UniChannel<FtI>, FtOC, bsg_fifo_tracker, <els_p>, true,}
impl_custom_inst! {UniChannel<BcudbI>, BcudbOC, bsg_counter_up_down, <max_val_p, init_val_p>, true,}
impl_custom_inst! {RdrArbIC, RdrArbOC, bsg_round_robin_arb, <inputs_p>, true,}
impl_custom_inst! {UniChannel<BigRamI>, BigRamOC, bsg_mem_1rw_sync, <width_p, els_p>, true,}
impl_custom_inst! {VrChannel<I>, OC, bsg_1_to_n_tagged_fifo, <width_p, num_out_p, els_pm, unbuffered_mask_p>, true,}

pub fn m() -> Module<IC, OC> {
    composite::<(IC, FeedbackC), (OC, FeedbackC), _>(
        "bsg_1_to_n_tagged_fifo_shared",
        Some("i"),
        Some("o"),
        |(input, feedback), k| {
            let (input, input_fwd) = input.clone_uni(k);

            let write_req = input_fwd.clone().map(k, |input| input.valid);

            let b1nt_o = input.map(k, |input| B1ntIProj { tag: input.tag }.into()).bsg_1_to_n_tagged::<NUM_OUT_P>(
                k,
                "b1nt",
                Some("i"),
                Some("o"),
            );

            let enque = b1nt_o.array_map(k, "enque", |ch, k| ch.fire(k).1);

            let yumi = feedback.map(k, |input| input.yumi).slice(k);

            let (ft_o, credits_avail) = enque
                .array_zip(yumi)
                .array_map(k, "bufd", |(ch, yumi), k| {
                    let ft_i = ch.map(k, |input| FtIProj { enq: input, deq: false.into() }.into());

                    let ft_o = ft_i.bsg_fifo_tracker::<ELS_P>(k, "ft", Some("i"), Some("o"));

                    let bcudb_o = yumi
                        .map(k, |input| BcudbIProj { up: input, down: false.into() }.into())
                        .bsg_counter_up_down::<BUFFERING_P, BUFFERING_P>(k, "bcudb", Some("i"), Some("o"));

                    let credits_avail = bcudb_o.map(k, |els| els.any());

                    (ft_o, credits_avail)
                })
                .unzip();

            let reqs = ft_o.array_zip(credits_avail).array_map(k, "rdr_arb_i", |(ft_o, credits_avail), k| {
                ft_o.zip(k, credits_avail).map(k, |input| (!input.0.empty) & input.1)
            });

            let reqs = reqs.concat(k);

            let rdr_arb_i = RdrArbIC { grants_en: UniChannel::source(k, true.into()), reqs };

            let rdr_arb_o = rdr_arb_i.bsg_round_robin_arb::<NUM_OUT_P>(k, "rdr_arb", Some("i"), Some("o"));

            let (_, read_req_fwd) = rdr_arb_o.tag.clone_uni(k);

            let big_ram_re = write_req.clone().zip(k, read_req_fwd.clone()).map(k, |input| {
                let (write_req, read_req_fwd) = *input;
                !write_req & read_req_fwd.valid
            });

            let sram_data_lo = input_fwd
                .zip3(k, write_req, read_req_fwd.clone())
                .map(k, |input| {
                    let (input, write_req, read_req_fwd) = *input;
                    BigRamIProj { data: input.inner.data, v: read_req_fwd.valid, w: write_req }.into()
                })
                .bsg_mem_1rw_sync::<WIDTH_P, ELS_P>(k, "big_ram", Some("i"), Some("data_o"));

            let big_ram_re_r = big_ram_re.buffer(k, false.into());
            let read_req_fwd_r = read_req_fwd.buffer(k, Expr::invalid());

            let output = big_ram_re_r
                .zip3(k, read_req_fwd_r, sram_data_lo)
                .map(k, |input| {
                    let (v, tag_fwd, data) = *input;
                    Expr::<Valid<_>>::new(v, IProj { tag: tag_fwd.inner, data }.into())
                })
                .into_vr(k)
                .bsg_1_to_n_tagged_fifo::<WIDTH_P, NUM_OUT_P, ELS_P, 0>(k, "b1ntf", Some("i"), Some("o"));

            let (output, yumi) = output.array_map(k, "yumi", |ch, k| ch.fire(k)).unzip();

            let feedback = yumi.concat(k).map(k, |input| FeedbackProj { yumi: input }.into());

            (output, feedback)
        },
    )
    .loop_feedback()
    .build()
}
