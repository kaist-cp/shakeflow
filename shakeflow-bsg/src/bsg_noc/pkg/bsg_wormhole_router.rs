#[doc(hidden)]
pub const STRICT_X: [[usize; 3]; 2] = [
    [
        // EWP (input)
        0b011, // E
        0b101, // W
        0b111, // P (output)
    ],
    [
        // EWP (output)
        0b011, // E
        0b101, // W
        0b111, // P (input)
    ],
];

// For testing only; you should never build a machine like this
#[doc(hidden)]
pub const X_ALLOW_LOOPBACK: [[usize; 3]; 2] = [
    [
        // EWP (input), it will deadlock
        0b100, // E
        0b010, // W
        0b000, // P (output)
    ],
    [
        // EWP (output)
        0b111, // E
        0b111, // W
        0b111, // P (input)
    ],
];

#[doc(hidden)]
pub const STRICT_XY: [[usize; 5]; 2] = [
    [
        // SNEWP (input)
        0b01111, // S
        0b10111, // N
        0b00011, // E
        0b00101, // W
        0b11111, // P (output)
    ],
    [
        // SNEWP (output)
        0b01001, // S
        0b10001, // N
        0b11011, // E
        0b11101, // W
        0b11111, // P (input)
    ],
];

#[doc(hidden)]
pub const STRICT_YX: [[usize; 5]; 2] = [
    [
        // SNEWP (input)
        0b01001, // S
        0b10001, // N
        0b11011, // E
        0b11101, // W
        0b11111, // P (output)
    ],
    [
        // SNEWP (output)
        0b01111, // S
        0b10111, // N
        0b00011, // E
        0b00101, // W
        0b11111, // P (input)
    ],
];

// These are "OR-in" machines, that are intended to be layered upon StrictYX or StrixtXY.
#[doc(hidden)]
pub const XY_ALLOW_S: [[usize; 5]; 2] = [
    [
        // SNEWP (input)
        0b00000, // S
        0b00000, // N
        0b10000, // E
        0b10000, // W
        0b00000, // P (output)
    ],
    [
        // SNEWP (output)
        0b00110, // S
        0b00000, // N
        0b00000, // E
        0b00000, // W
        0b00000, // P (input)
    ],
];

#[doc(hidden)]
pub const XY_ALLOW_N: [[usize; 5]; 2] = [
    [
        // SNEWP (input)
        0b00000, // S
        0b00000, // N
        0b01000, // E
        0b01000, // W
        0b00000, // P (output)
    ],
    [
        // SNEWP (output)
        0b00000, // S
        0b00110, // N
        0b00000, // E
        0b00000, // W
        0b00000, // P (input)
    ],
];

#[doc(hidden)]
pub const YX_ALLOW_W: [[usize; 5]; 2] = [
    [
        // SNEWP (input)
        0b00010, // S
        0b00010, // N
        0b00000, // E
        0b00000, // W
        0b00000, // P (output)
    ],
    [
        // SNEWP (output)
        0b00000, // S
        0b00000, // N
        0b00000, // E
        0b11000, // W
        0b00000, // P (input)
    ],
];

#[doc(hidden)]
pub const YX_ALLOW_E: [[usize; 5]; 2] = [
    [
        // SNEWP (input)
        0b00100, // S
        0b00100, // N
        0b00000, // E
        0b00000, // W
        0b00000, // P (output)
    ],
    [
        // SNEWP (output)
        0b00000, // S
        0b00000, // N
        0b11000, // E
        0b00000, // W
        0b00000, // P (input)
    ],
];
