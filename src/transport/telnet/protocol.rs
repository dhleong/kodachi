pub const SE: u8 = 240;
pub const SB: u8 = 250;
pub const WILL: u8 = 251;
pub const WONT: u8 = 252;
pub const DO: u8 = 253;
pub const DONT: u8 = 254;

// Interpret As Command
pub const IAC: u8 = 255;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NegotiationType {
    Will,
    Wont,
    Do,
    Dont,
}

impl NegotiationType {
    pub fn from_byte(byte: u8) -> Self {
        match byte {
            WILL => NegotiationType::Will,
            WONT => NegotiationType::Wont,
            DO => NegotiationType::Do,
            DONT => NegotiationType::Dont,
            _ => panic!("Not a negotiation type: {:?}", byte),
        }
    }

    pub fn byte(&self) -> u8 {
        match self {
            NegotiationType::Will => WILL,
            NegotiationType::Wont => WONT,
            NegotiationType::Do => DO,
            NegotiationType::Dont => DONT,
        }
    }
}

macro_rules! declare_type {
    (
        $type_name:ident {
            $($name:ident => $byte_value_name:expr,)*
        }
    ) => {
        #[allow(dead_code)]
        #[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
        pub enum $type_name {
            $($name),*,
            Unknown(u8),
        }

        impl $type_name {
            #[allow(dead_code)]
            pub fn from_byte(byte: u8) -> Self {
                match byte {
                    $($byte_value_name => $type_name::$name),*,
                    _ => $type_name::Unknown(byte),
                }
            }

            #[allow(dead_code)]
            pub fn byte(&self) -> u8 {
                match self {
                    $($type_name::$name => $byte_value_name),*,
                    $type_name::Unknown(byte) => *byte,
                }
            }
        }
    };
}

declare_type!(TelnetCommand {
    GoAhead => 249,
});

declare_type!(TelnetOption {
    Echo => 1,
    SuppressGoAhead => 3,
    TermType => 24,
    // Negotiate About Window Size
    Naws => 31,
    Charset => 42,
    MSDP => 69,
    MCCP2 => 86,
    MCCP3 => 87,
    MSP => 90,
    GMCP => 201,
});
