/// Ruche directions
#[derive(Debug, Clone, Copy)]
pub enum RucheDirs {
    RW = 5,
    RE = 6,
    RN = 7,
    RS = 8,
}

/// Routing matrices when dimension is 2 and XY order is 1.
pub const STRICT_XY: [usize; 5] = [
    // SNEWP (input)
    0b01111, // S
    0b10111, // N
    0b00011, // E
    0b00101, // W
    0b11111, // P (output)
];

/// Routing matrices when dimension is 2 and XY order is 0.
pub const STRICT_YX: [usize; 5] = [
    // SNEWP (input)
    0b01001, // S
    0b10001, // N
    0b11011, // E
    0b11101, // W
    0b11111, // P (output)
];

/// Routing matrices when dimension is 3 and XY order is 1.
pub const HALF_RUCHE_X_STRICT_XY: [usize; 7] = [
    // RE,RW,SNEWP (input)
    0b0100001, // RE
    0b1000001, // RW
    0b0001111, // S
    0b0010111, // N
    0b0100011, // E
    0b1000101, // W
    0b0011111, // P (output)
];

/// Routing matrices when dimension is 3 and XY order is 0.
pub const HALF_RUCHE_X_STRICT_YX: [usize; 7] = [
    // RE,RW,SNEWP (input)
    0b0100010, // RE
    0b1000100, // RW
    0b0001001, // S
    0b0010001, // N
    0b0011011, // E
    0b0011101, // W
    0b1111111, // P (output)
];

/// Routing matrices when dimension is 4 and XY order is 1.
pub const FULL_RUCHE_STRICT_XY: [usize; 9] = [
    // RS,RN,RE,RW,SNEWP (input)
    0b010001000, // RS
    0b100010000, // RN
    0b000100001, // RE
    0b001000001, // RW
    0b000001111, // S
    0b000010111, // N
    0b000100011, // E
    0b001000101, // W
    0b110011111, // P (output)
];

/// Routing matrices when dimension is 4 and XY order is 0.
pub const FULL_RUCHE_STRICT_YX: [usize; 9] = [
    // RS,RN,RE,RW,SNEWP (input)
    0b010000001, // RS
    0b100000001, // RN
    0b000100010, // RE
    0b001000100, // RW
    0b010001001, // S
    0b100010001, // N
    0b000011011, // E
    0b000011101, // W
    0b001111111, // P (output)
];

/// Routing matrices when dimension is 3 and XY order is 1 and depopulated.
pub const HALF_RUCHE_X_FULLY_POPULATED_STRICT_XY: [usize; 7] = [
    // RE,RW,SNEWP (input)
    0b0100001, // RE
    0b1000001, // RW
    0b1101111, // S
    0b1110111, // N
    0b0100011, // E
    0b1000101, // W
    0b1111111, // P (output)
];

/// Routing matrices when dimension is 3 and XY order is 0 and depopulated.
pub const HALF_RUCHE_X_FULLY_POPULATED_STRICT_YX: [usize; 7] = [
    // RE,RW,SNEWP (input)
    0b0111011, // RE
    0b1011101, // RW
    0b0001001, // S
    0b0010001, // N
    0b0011011, // E
    0b0011101, // W
    0b1111111, // P (output)
];

/// Routing matrices when dimension is 4 and XY order is 1 and depopulated.
pub const FULL_RUCHE_FULLY_POPULATED_STRICT_XY: [usize; 9] = [
    // RS,RN,RE,RW,SNEWP (input)
    0b011101111, // RS
    0b101110111, // RN
    0b000100001, // RE
    0b001000001, // RW
    0b001101111, // S
    0b001110111, // N
    0b000100011, // E
    0b001000101, // W
    0b111111111, // P (output)
];

/// Routing matrices when dimension is 4 and XY order is 0 and depopulated.
pub const FULL_RUCHE_FULLY_POPULATED_STRICT_YX: [usize; 9] = [
    // RS,RN,RE,RW,SNEWP (input)
    0b010000001, // RS
    0b100000001, // RN
    0b110111011, // RE
    0b111011101, // RW
    0b010001001, // S
    0b100010001, // N
    0b110011011, // E
    0b110011101, // W
    0b111111111, // P (output)
];
