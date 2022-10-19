//! Based on US Patent # 4,486,739 (expired)
//! Byte Oriented DC Balanced 8B/10B Partitioned Block Transmission Code
//! Author: Franaszek et al.
//!
//! <https://patentimages.storage.googleapis.com/67/2d/ad/0258c2f0d807bf/US4486739.pdf>

#![allow(non_snake_case)]

use shakeflow::*;
use shakeflow_std::*;

/// Ingress signal.
#[derive(Debug, Clone, Signal)]
pub struct I {
    data: Bits<U<10>>,
    rd: bool,
}

/// Egress signal.
#[derive(Debug, Clone, Signal)]
pub struct E {
    data: Bits<U<8>>,
    k: bool,
    rd: bool,
    data_err: bool,
    rd_err: bool,
}

/// Ingress channel.
pub type IC = UniChannel<I>;
/// Egress channel.
pub type EC = UniChannel<E>;

pub fn logic(input: Expr<'_, I>) -> Expr<'_, E> {
    let data_i = input.data;
    let rd_i = input.rd;

    let A = data_i[0];
    let B = data_i[1];
    let C = data_i[2];
    let D = data_i[3];
    let E = data_i[4];
    let I = data_i[5];
    let F = data_i[6];
    let G = data_i[7];
    let H = data_i[8];
    let J = data_i[9];

    // Commonly found functions (some in patent, others are not)
    let AxorB = A ^ B;
    let AandB = A & B;
    let NAandNB = !A & !B;

    let CxorD = C ^ D;
    let CandD = C & D;
    let NCandND = !C & !D;

    let ExnorI = !(E ^ I);
    let EandI = E & I;
    let NEandNI = !E & !I;

    let FxorG = F ^ G;
    let FandG = F & G;
    let NFandNG = !F & !G;

    let HxorJ = H ^ J;
    let HandJ = H & J;
    let NHandNJ = !H & !J;

    // From FIG. 10
    let P22 = (AandB & NCandND) | (CandD & NAandNB) | (AxorB & CxorD);
    let P13 = (AxorB & NCandND) | (CxorD & NAandNB);
    let P31 = (AxorB & CandD) | (CxorD & AandB);

    // From FIG. 11
    let N0 = P22 & A & C & ExnorI;
    let N1 = P22 & !A & !C & ExnorI;
    let N2 = P22 & B & C & ExnorI;
    let N3 = P22 & !B & !C & ExnorI;
    let N4 = NAandNB & NEandNI;
    let N5 = AandB & EandI;
    let N6 = P13 & D & EandI;
    let N7 = P13 & !I;
    let N8 = P13 & !E;
    let N9 = P31 & I;

    let N10 = CandD & EandI;
    let N11 = NCandND & NEandNI;
    let N12 = !E & I & G & HandJ;
    let N13 = E & !I & !G & NHandNJ;

    let k_o = (N10 | N11) | (N12 & P13) | (N13 & P31);

    // From FIG. 12
    let M0 = N1 | N8;
    let M1 = N5 | N11 | N9;
    let M2 = N9 | N2 | N6;
    let M3 = N0 | N8;
    let M4 = N8 | N11 | N4;
    let M5 = N1 | N7;
    let M6 = N6 | N3;

    let T0 = M6 | M0 | M1;
    let T1 = M1 | M3 | M2;
    let T2 = M2 | M0 | M4;
    let T3 = M1 | M3 | M6;
    let T4 = M5 | M4 | M6;

    let data_o_0 = A ^ T0;
    let data_o_1 = B ^ T1;
    let data_o_2 = C ^ T2;
    let data_o_3 = D ^ T3;
    let data_o_4 = E ^ T4;

    // From FIG. 13
    let N14 = G & HandJ;
    let N15 = HandJ & F;
    let N16 = FandG & J;
    let N17 = NFandNG & !H;
    let N18 = NFandNG & HandJ;
    let N19 = !F & NHandNJ;
    let N20 = NHandNJ & !G;
    let N21 = !HandJ & !NHandNJ & N11;

    let M7 = N14 | N15 | N21;
    let M8 = N16 | N17 | N18;
    let M9 = N19 | N21 | N20;
    let M10 = N20 | N15 | N21;

    let T5 = M7 | M8;
    let T6 = M8 | M9;
    let T7 = M8 | M10;

    let data_o_5 = F ^ T5;
    let data_o_6 = G ^ T6;
    let data_o_7 = H ^ T7;

    // Everything else is not found in the patent

    let rd6p = (P31 & !NEandNI) | (P22 & EandI); // 5b/6b code disparity +2
    let rd6n = (P13 & !EandI) | (P22 & NEandNI); // 5b/6b code disparity -2
    let rd4p = (FxorG & HandJ) | (HxorJ & FandG); // 3b/4b code disparity +2
    let rd4n = (FxorG & NHandNJ) | (HxorJ & NFandNG); // 3b/4b code disparity -2

    let rd_o = !NHandNJ
        & (rd4p
            | HandJ
            | (((D | !NEandNI)
                & ((rd_i & P31) | ((rd_i | !P13) & EandI) | (((rd_i & P22) | P31) & !(NEandNI)) | (D & EandI)))
                & ((FandG & NHandNJ) | N18 | (FxorG & HxorJ))));

    let data_err_o = (NAandNB & NCandND)
        | (AandB & CandD)
        | (NFandNG & NHandNJ)
        | (FandG & HandJ)
        | (EandI & FandG & H)
        | (NEandNI & N17)
        | (E & !I & N14)
        | (!E & I & N20)
        | (!P31 & N13)
        | (!P13 & N12)
        | (N7 & !E)
        | (N9 & E)
        | (FandG & NHandNJ & rd6p)
        | (N18 & rd6n)
        | (N10 & N17)
        | (N11 & FandG & H)
        | (rd6p & rd4p)
        | (rd6n & rd4n)
        | (AandB & C & NEandNI & (NFandNG | rd4n))
        | (NAandNB & !C & EandI & (FandG | rd4p))
        | (((EandI & N20) | (NEandNI & N14)) & !(CandD & E) & !(NCandND & !E));

    // Running disparity errors detection
    let rd_err_o = (rd6p & rd4p) | (rd6n & rd4n) | // Delta disparity check
                            (rd_i & rd6p) | (!rd_i & rd6n) | // Disparity check for 5b/6b code
                            (rd_i & !rd6n & FandG) | (!rd_i & !rd6p & NFandNG) | // Disparity check for 3b/4b code
                            (rd_i & !rd6n & rd4p) | (!rd_i & !rd6p & rd4n) | // Resulting disparity check
                            (rd_i & AandB & C) | (!rd_i & NAandNB & !C); // Additional check

    let data_o =
        Expr::<Bits<U<8>>>::from([data_o_0, data_o_1, data_o_2, data_o_3, data_o_4, data_o_5, data_o_6, data_o_7]);

    EProj { data: data_o, k: k_o, rd: rd_o, data_err: data_err_o, rd_err: rd_err_o }.into()
}

pub fn m() -> Module<IC, EC> {
    composite::<IC, EC, _>("bsg_8b10b_decode_comb", Some("i"), Some("o"), |input, k| input.map(k, logic)).build()
}
