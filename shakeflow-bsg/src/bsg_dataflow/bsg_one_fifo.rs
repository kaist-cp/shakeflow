use shakeflow::*;
use shakeflow_std::*;

pub type IC<Width: Num, const P: Protocol> = VrChannel<Bits<Width>, P>;
pub type OC<Width: Num> = VrChannel<Bits<Width>>;

pub fn m<Width: Num, const P: Protocol>() -> Module<IC<Width, P>, OC<Width>> {
    composite::<IC<Width, P>, OC<Width>, _>("bsg_one_fifo", Some("i"), Some("o"), |input, k| input.buffer(k)).build()
}
