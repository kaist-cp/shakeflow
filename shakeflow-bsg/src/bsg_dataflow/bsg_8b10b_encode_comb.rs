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
    data: Bits<U<8>>,
    k: bool,
    rd: bool,
}

/// Egress signal.
#[derive(Debug, Clone, Signal)]
pub struct E {
    data: Bits<U<10>>,
    rd: bool,
    kerr: bool,
}

/// Ingress channel.
pub type IC = UniChannel<I>;
/// Egress channel.
pub type EC = UniChannel<E>;

pub fn m() -> Module<IC, EC> {
    composite::<IC, EC, _>("bsg_8b10b_encode_comb", Some("i"), Some("o"), |input, k| {
        input.map(k, |input| {
            let data_i = input.data;
            let k_i = input.k;
            let rd_i = input.rd;

            let A = data_i[0];
            let B = data_i[1];
            let C = data_i[2];
            let D = data_i[3];
            let E = data_i[4];
            let F = data_i[5];
            let G = data_i[6];
            let H = data_i[7];

            // From FIG. 3
            let AxorB = A ^ B;
            let CxorD = C ^ D;
            let AandB = A & B;
            let CandD = C & D;
            let NAandNB = !A & !B;
            let NCandND = !C & !D;

            let L22 = (AandB & NCandND) | (CandD & NAandNB) | (AxorB & CxorD);
            let L40 = AandB & CandD;
            let L04 = NAandNB & NCandND;
            let L13 = (AxorB & NCandND) | (CxorD & NAandNB);
            let L31 = (AxorB & CandD) | (CxorD & AandB);

            // From FIG. 4
            let FxorG = F ^ G;
            let FandG = F & G;
            let NFandNG = !F & !G;
            let NFandNGandNH = NFandNG & !H;
            let FxorGandK = FxorG & k_i;
            let FxorGandNH = FxorG & !H;
            let FandGandH = FandG & H;

            let S = (rd_i & L31 & D & !E) | (!rd_i & L13 & !D & E);

            // Form FIG. 5
            let T0 = L13 & D & E; // Intermediate net

            let PDM1S6 = T0 | (!L22 & !L31 & !E);
            let ND0S6 = PDM1S6;
            let PD0S6 = (E & !L22 & !L13) | k_i;
            let NDM1S6 = (L31 & !D & !E) | PD0S6;
            let NDM1S4 = FandG;
            let ND0S4 = NFandNG;
            let PDM1S4 = NFandNG | FxorGandK;
            let PD0S4 = FandGandH;

            // From FIG. 6
            let COMPLS6 = (NDM1S6 & rd_i) | (!rd_i & PDM1S6);
            let NDL6 = (PD0S6 & !COMPLS6) | (COMPLS6 & ND0S6) | (!ND0S6 & !PD0S6 & rd_i);
            let COMPLS4 = (NDM1S4 & NDL6) | (!NDL6 & PDM1S4);

            let rd_o = (NDL6 & !PD0S4 & !ND0S4) | (ND0S4 & COMPLS4) | (!COMPLS4 & PD0S4);

            // From FIG. 7
            let N0 = A;
            let N1 = (!L40 & B) | L04;
            let N2 = (L04 | C) | T0;
            let N3 = D & !L40;
            let N4 = (!T0 & E) | (!E & L13);
            let N5 = (!E & L22) | (L22 & k_i) | (L04 & E) | (E & L40) | (E & L13 & !D);

            let data_o_0 = N0 ^ COMPLS6;
            let data_o_1 = N1 ^ COMPLS6;
            let data_o_2 = N2 ^ COMPLS6;
            let data_o_3 = N3 ^ COMPLS6;
            let data_o_4 = N4 ^ COMPLS6;
            let data_o_5 = N5 ^ COMPLS6;

            // From FIG. 8
            let T1 = (S & FandGandH) | (FandGandH & k_i); // Intermediate net

            let N6 = !(!F | T1);
            let N7 = G | NFandNGandNH;
            let N8 = H;
            let N9 = T1 | FxorGandNH;

            let data_o_6 = N6 ^ COMPLS4;
            let data_o_7 = N7 ^ COMPLS4;
            let data_o_8 = N8 ^ COMPLS4;
            let data_o_9 = N9 ^ COMPLS4;

            // Not in patent
            let kerr_o = k_i & !(NAandNB & CandD & E) & !(FandGandH & E & L31);

            let data_o = Expr::<Bits<U<10>>>::from([
                data_o_0, data_o_1, data_o_2, data_o_3, data_o_4, data_o_5, data_o_6, data_o_7, data_o_8, data_o_9,
            ]);

            EProj { data: data_o, rd: rd_o, kerr: kerr_o }.into()
        })
    })
    .build()
}
