use shakeflow::*;
use shakeflow_std::*;

#[derive(Debug, Clone, Signal)]
#[width(6)]
pub enum Instruction {
    #[encode(0o7)]
    Add,
    #[encode(4)]
    Sub,
    #[encode(0b00101)]
    Mult,
    #[encode(0xA)]
    Div,
    #[encode(0b10_1010)]
    Nop,
    Rem,
}

pub fn m() -> Module<UniChannel<Instruction>, UniChannel<u8>> {
    composite::<UniChannel<Instruction>, UniChannel<u8>, _>("enum", Some("i"), Some("o"), |i, k| {
        i.map(k, |inst| {
            inst.case(
                vec![
                    (Instruction::Add.into(), 0.into()),
                    (Instruction::Sub.into(), 1.into()),
                    (Instruction::Mult.into(), 2.into()),
                    (Instruction::Div.into(), 3.into()),
                    (Instruction::Rem.into(), 4.into()),
                    (Instruction::Nop.into(), 5.into()),
                ],
                Some(4.into()),
            )
        })
    })
    .build()
}
