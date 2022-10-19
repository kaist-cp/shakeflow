use shakeflow::*;

/// Direction type.
#[derive(Debug, Clone, Signal)]
#[width(3)]
pub enum Dirs {
    P,
    W,
    E,
    N,
    S,
}
