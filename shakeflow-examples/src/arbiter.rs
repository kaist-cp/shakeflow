use shakeflow::*;
use shakeflow_std::*;

pub const PORTS: usize = 4;
pub const ARB_TYPE_ROUND_ROBIN: bool = false;
pub const ARB_BLOCK: bool = false;
pub const ARB_BLOCK_ACK: bool = true;
pub const ARB_LSB_HIGH_PRIORITY: usize = 0;
const CL_PORTS: usize = clog2(PORTS);

#[derive(Debug, Clone, Signal)]
pub struct ArbiterInput {
    request: Bits<U<PORTS>>,
    acknowledge: Bits<U<PORTS>>,
}

#[derive(Debug, Clone, Signal)]
pub struct ArbiterOutput {
    grant: Bits<U<PORTS>>,
    grant_valid: bool,
    grant_encoded: Bits<Log2<U<PORTS>>>,
}

type ArbiterInputChannel = UniChannel<ArbiterInput>;
type ArbiterOutputChannel = UniChannel<ArbiterOutput>;

#[derive(Debug, Clone, Signal)]
pub struct PriorityEncoderInput<const N: usize> {
    unencoded: Bits<U<N>>,
}

#[derive(Debug, Clone, Signal)]
pub struct PriorityEncoderOutput<const N: usize> {
    valid: bool,
    encoded: Bits<Log2<U<PORTS>>>,
    unencoded: Bits<U<N>>,
}

// pub type PriorityEncoderInputChannel<const N: usize> = UniChannel<PriorityEncoderInput<N>>;
pub type PriorityEncoderOutputChannel<const N: usize> = UniChannel<PriorityEncoderOutput<N>>;

#[derive(Debug, Clone, Signal)]
struct State {
    grant_reg: Bits<U<PORTS>>,
    grant_valid_reg: bool,
    grant_encoded_reg: Bits<Log2<U<PORTS>>>,
    mask_reg: Bits<U<PORTS>>,
}

pub fn arbiter() -> Module<ArbiterInputChannel, ArbiterOutputChannel> {
    composite::<(ArbiterInputChannel, UniChannel<Bits<U<PORTS>>>), (ArbiterOutputChannel, UniChannel<Bits<U<PORTS>>>), _>(
        "arbiter",
        None,
        None,
        |input, k| {
            let (input, mask_reg) = input;

            let request = input.clone().map(k, |input| input.request);

            let priority_encoder_inst_input = request.clone()
                .map(k, |unencoded| {
                    PriorityEncoderInputProj {
                        unencoded,
                    }
                    .into()
                });

            let priority_encoder_inst_output: PriorityEncoderOutputChannel<PORTS> = priority_encoder_inst_input
                .module_inst(
                    k,
                    "priority_encoder",
                    "priority_encoder_inst",
                    vec![
                        ("WIDTH", PORTS),
                        ("LSB_HIGH_PRIORITY", ARB_LSB_HIGH_PRIORITY),
                    ],
                    false,
                    Some("input"),
                    Some("output"),
                );

            let priority_encoder_masked_input = request
                .zip(k, mask_reg)
                .map(k, |input| {
                    let (request, mask_reg) = *input;
                    PriorityEncoderInputProj {
                        unencoded: request & mask_reg,
                    }
                    .into()
                });

            let priority_encoder_masked_output: PriorityEncoderOutputChannel<PORTS> = priority_encoder_masked_input
                .module_inst(
                    k,
                    "priority_encoder",
                    "priority_encoder_masked",
                    vec![
                        ("WIDTH", PORTS),
                        ("LSB_HIGH_PRIORITY", ARB_LSB_HIGH_PRIORITY),
                    ],
                    false,
                    Some("input"),
                    Some("output"),
                );

            let output_mask_reg = input
                .zip3(k, priority_encoder_inst_output, priority_encoder_masked_output)
                .fsm_map(
                    k,
                    None,
                    StateProj {
                        grant_reg: [false; PORTS].into(),
                        grant_valid_reg: false.into(),
                        grant_encoded_reg: 0.into(),
                        mask_reg: [false; PORTS].into()
                    }
                    .into(),
                    |input, state| {
                        // Projections
                        let (input, priority_encoder_inst_output, priority_encoder_masked_output) = *input;

                        let request = input.request;
                        let acknowledge = input.acknowledge;

                        let request_valid = priority_encoder_inst_output.valid;
                        let request_index = priority_encoder_inst_output.encoded;
                        let request_mask = priority_encoder_inst_output.unencoded;

                        let masked_request_valid = priority_encoder_masked_output.valid;
                        let masked_request_index = priority_encoder_masked_output.encoded;
                        let masked_request_mask = priority_encoder_masked_output.unencoded;

                        let grant_reg = state.grant_reg;
                        let grant_valid_reg = state.grant_valid_reg;
                        let grant_encoded_reg = state.grant_encoded_reg;
                        let mask_reg = state.mask_reg;

                        let grant_valid = grant_valid_reg;

                        let state_next = select! {
                            Expr::from(ARB_BLOCK) & !Expr::from(ARB_BLOCK_ACK) & !(grant_reg & request).is_eq(0.into()) => StateProj {
                                grant_valid_reg,
                                grant_reg,
                                grant_encoded_reg,
                                mask_reg
                            }
                            .into(),
                            Expr::from(ARB_BLOCK) & Expr::from(ARB_BLOCK_ACK) & grant_valid & (grant_reg & acknowledge).is_eq(0.into()) => StateProj {
                                grant_valid_reg,
                                grant_reg,
                                grant_encoded_reg,
                                mask_reg
                            }
                            .into(),
                            request_valid & Expr::from(ARB_TYPE_ROUND_ROBIN) & masked_request_valid & !Expr::from(ARB_LSB_HIGH_PRIORITY as u64).repr().is_eq(0.into()) => StateProj {
                                grant_valid_reg: true.into(),
                                grant_reg: masked_request_mask,
                                grant_encoded_reg: masked_request_index,
                                mask_reg: Expr::from(true).repeat::<U<PORTS>>() << (masked_request_index.resize::<U<CL_PORTS>>() + 1.into()).resize::<U<CL_PORTS>>()
                            }
                            .into(),
                            request_valid & Expr::from(ARB_TYPE_ROUND_ROBIN) & masked_request_valid => StateProj {
                                grant_valid_reg: true.into(),
                                grant_reg: masked_request_mask,
                                grant_encoded_reg: masked_request_index,
                                mask_reg: Expr::from(true).repeat::<U<PORTS>>() >> (Expr::from(PORTS as u8).repr() - masked_request_index.resize()).resize::<U<CL_PORTS>>()
                            }
                            .into(),
                            request_valid & Expr::from(ARB_TYPE_ROUND_ROBIN) & !Expr::from(ARB_LSB_HIGH_PRIORITY as u64).repr().is_eq(0.into()) => StateProj {
                                grant_valid_reg: true.into(),
                                grant_reg: request_mask,
                                grant_encoded_reg: request_index,
                                mask_reg: Expr::from(true).repeat::<U<PORTS>>() << (request_index.resize::<U<CL_PORTS>>() + 1.into()).resize::<U<CL_PORTS>>(),
                            }
                            .into(),
                            request_valid => StateProj {
                                grant_valid_reg: true.into(),
                                grant_reg: request_mask,
                                grant_encoded_reg: request_index,
                                mask_reg: (Expr::from(PORTS as u8).repr() - request_index.resize()).resize()
                            }
                            .into(),
                            default => StateProj {
                                grant_valid_reg: false.into(),
                                grant_reg: 0.into(),
                                grant_encoded_reg: 0.into(),
                                mask_reg
                            }
                            .into(),
                        };

                        let output = ArbiterOutputProj {
                            grant: grant_reg,
                            grant_valid: grant_valid_reg,
                            grant_encoded: grant_encoded_reg
                        }
                        .into();

                        ((output, mask_reg).into(), state_next)
                    }
                );

            let output = output_mask_reg.clone()
                .map(k, |output_mask_reg| {
                    let (output, _) = *output_mask_reg;
                    output
                });

            let mask_reg = output_mask_reg
                .map(k, |output_mask_reg| {
                    let (_, mask_reg) = *output_mask_reg;
                    mask_reg
                });

            (output, mask_reg)
        }
    )
    .loop_feedback()
    .build()
}
