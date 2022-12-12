// Go Ahead
pub const GA: u8 = 249;

pub const SE: u8 = 240;
pub const SB: u8 = 250;
pub const WILL: u8 = 251;
pub const WONT: u8 = 252;
pub const DO: u8 = 253;
pub const DONT: u8 = 254;

// Interpret As Command
pub const IAC: u8 = 255;

pub mod options {
    // Terminal Type
    pub const TTYPE: u8 = 24;

    // Negotiate About Window Size
    pub const NAWS: u8 = 31;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TelnetCommand {
    GoAhead,
    Unknown(u8),
}

impl TelnetCommand {
    pub fn from_byte(byte: u8) -> TelnetCommand {
        match byte {
            GA => TelnetCommand::GoAhead,
            _ => TelnetCommand::Unknown(byte),
        }
    }
}

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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TelnetOption {
    Ttype,
    Naws,
    Unknown(u8),
}

impl TelnetOption {
    pub fn from_byte(byte: u8) -> Self {
        match byte {
            options::TTYPE => TelnetOption::Ttype,
            options::NAWS => TelnetOption::Naws,
            _ => TelnetOption::Unknown(byte),
        }
    }
}
