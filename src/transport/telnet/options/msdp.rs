use std::{collections::HashMap, io};

use async_trait::async_trait;

use crate::{
    net::{
        readable::Readable,
        writable::{ObjectWriteStream, Writable},
    },
    transport::telnet::{
        processor::TelnetEvent,
        protocol::{NegotiationType, TelnetOption},
    },
};

use super::{negotiator::OptionsNegotiatorBuilder, DynWriteStream, TelnetOptionHandler};

const MSDP_VAR: u8 = 1;
const MSDP_VAL: u8 = 2;
const MSDP_TABLE_OPEN: u8 = 3;
const MSDP_TABLE_CLOSE: u8 = 4;
const MSDP_ARRAY_OPEN: u8 = 5;
const MSDP_ARRAY_CLOSE: u8 = 6;

struct MsdpVar(MsdpName, MsdpVal);

impl Writable for MsdpVar {
    fn write<S: io::Write>(self, stream: &mut S) -> io::Result<()> {
        self.0.write(stream)?;
        self.1.write(stream)
    }
}

#[derive(Debug)]
pub enum MsdpName {
    List,
    Report,
    Unreport,
    Other(String),
}

impl Writable for MsdpName {
    fn write<S: io::Write>(self, stream: &mut S) -> io::Result<()> {
        stream.write_all(&[MSDP_VAR])?;
        match self {
            MsdpName::Other(s) => stream.write_all(s.as_bytes()),
            named => {
                let s = format!("{:?}", named);
                stream.write_all(s.to_uppercase().as_bytes())
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MsdpVal {
    String(String),
    Array(Vec<MsdpVal>),
    FlatArray(Vec<MsdpVal>),
    Table(HashMap<String, MsdpVal>),
}

impl Writable for MsdpVal {
    fn write<S: io::Write>(self, stream: &mut S) -> io::Result<()> {
        stream.write_all(&[MSDP_VAL])?;
        match self {
            MsdpVal::String(s) => stream.write_all(&s.as_bytes()),
            MsdpVal::Array(items) => {
                stream.write_all(&[MSDP_ARRAY_OPEN])?;
                for item in items {
                    item.write(stream)?;
                }
                stream.write_all(&[MSDP_ARRAY_CLOSE])
            }
            MsdpVal::FlatArray(items) => {
                for item in items {
                    item.write(stream)?;
                }
                Ok(())
            }
            MsdpVal::Table(items) => {
                stream.write_all(&[MSDP_TABLE_OPEN])?;
                for (key, val) in items {
                    MsdpVar(MsdpName::Other(key), val).write(stream)?;
                }
                stream.write_all(&[MSDP_TABLE_CLOSE])
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum ReadableMsdpVal {
    None,
    Ok(MsdpVal),
    ArrayClose,
    TableClose,
}

impl Readable for ReadableMsdpVal {
    fn read<S: io::BufRead>(stream: &mut S) -> io::Result<Self> {
        let bytes = stream.fill_buf()?;
        let header = match bytes.get(0) {
            Some(byte) => *byte,
            None => return Ok(ReadableMsdpVal::None),
        };
        let next_byte = bytes.get(1).map(|byte| *byte);
        stream.consume(1);

        match header {
            MSDP_VAL | MSDP_VAR => {} // Normal; fall through to parsing below
            MSDP_ARRAY_CLOSE => {
                return Ok(ReadableMsdpVal::ArrayClose);
            }
            MSDP_TABLE_CLOSE => {
                return Ok(ReadableMsdpVal::TableClose);
            }
            unexpected => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Expected MSDP_VAL but got {}", unexpected),
                ));
            }
        }

        match next_byte {
            None => Ok(ReadableMsdpVal::None),
            Some(MSDP_ARRAY_OPEN) => {
                stream.consume(1);
                let mut vec = Vec::new();
                while let ReadableMsdpVal::Ok(val) = Self::read(stream)? {
                    vec.push(val);
                }
                Ok(ReadableMsdpVal::Ok(MsdpVal::Array(vec)))
            }
            Some(MSDP_TABLE_OPEN) => {
                stream.consume(1);
                let mut map = HashMap::new();
                loop {
                    let name = match Self::read(stream)? {
                        ReadableMsdpVal::Ok(MsdpVal::String(name)) => name,
                        ReadableMsdpVal::TableClose => {
                            break;
                        }
                        unexpected => {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                format!("Expected MSDP_VAR but got {:?}", unexpected),
                            ));
                        }
                    };
                    let value = match Self::read(stream)? {
                        ReadableMsdpVal::Ok(value) => value,
                        unexpected => {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                format!("Expected MSDP_VAL but got {:?}", unexpected),
                            ));
                        }
                    };
                    map.insert(name, value);
                }
                Ok(ReadableMsdpVal::Ok(MsdpVal::Table(map)))
            }
            Some(_) => {
                let buf = stream.fill_buf()?;
                for i in 0..buf.len() {
                    match buf[i] {
                        MSDP_VAR | MSDP_VAL | MSDP_ARRAY_CLOSE | MSDP_TABLE_CLOSE => {
                            let content = String::from_utf8_lossy(&buf[0..i]).to_string();
                            let value = MsdpVal::String(content);
                            stream.consume(i);
                            // TODO if the next byte is MSDP_VAL this is a FlatArray
                            return Ok(ReadableMsdpVal::Ok(value));
                        }
                        _ => continue,
                    };
                }
                todo!();
            }
        }
    }
}

pub enum MsdpEvent {
    Reset,
    UpdateVar(String, MsdpVal),
}

pub struct MsdpOptionHandler {}

impl MsdpOptionHandler {
    pub fn new() -> (Self, Option<usize>) {
        (MsdpOptionHandler {}, None)
    }
}

impl MsdpOptionHandler {
    fn reset(&self) {}
}

#[async_trait]
impl TelnetOptionHandler for MsdpOptionHandler {
    fn option(&self) -> TelnetOption {
        TelnetOption::MSDP
    }

    fn register(&self, negotiator: OptionsNegotiatorBuilder) -> OptionsNegotiatorBuilder {
        negotiator.accept_will(TelnetOption::MSDP)
    }

    async fn negotiate(
        &mut self,
        negotiation: NegotiationType,
        mut stream: DynWriteStream<'_>,
    ) -> io::Result<()> {
        match negotiation {
            NegotiationType::Wont => {
                self.reset();
            }

            NegotiationType::Will => {
                let to_send = MsdpVar(MsdpName::List, MsdpVal::String("COMMANDS".to_string()));
                let command = TelnetEvent::Subnegotiate(TelnetOption::MSDP, to_send.into_bytes());

                log::trace!(target: "telnet", ">> MSDP LIST COMMANDS");
                stream.write_object(command).await?;
            }

            _ => {}
        }
        Ok(())
    }

    async fn subnegotiate(
        &mut self,
        data: bytes::Bytes,
        _stream: DynWriteStream<'_>,
    ) -> io::Result<()> {
        log::trace!(target: "telnet", "<< MSDP (TODO) {:?}", data);
        // TODO Parse MSDP data and stash somewhere
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use bytes::{Buf, Bytes};

    use super::*;

    #[test]
    fn serialize_list_command_test() {
        let to_send = MsdpVar(MsdpName::List, MsdpVal::String("COMMANDS".to_string()));
        let bytes = to_send.into_bytes();
        assert_eq!(bytes, Bytes::from("\x01LIST\x02COMMANDS"));
    }

    #[test]
    fn read_simple_array_value_test() {
        let original = MsdpVal::Array(vec![
            MsdpVal::String("LIST".to_string()),
            MsdpVal::String("REPORT".to_string()),
        ]);
        let source = original.clone().into_bytes();

        let data = ReadableMsdpVal::read(&mut source.reader()).unwrap();
        assert_eq!(data, ReadableMsdpVal::Ok(original));
    }

    #[test]
    fn read_simple_table_value_test() {
        let original = MsdpVal::Table(HashMap::from([(
            "LIST".to_string(),
            MsdpVal::String("REPORT".to_string()),
        )]));
        let source = original.clone().into_bytes();

        let data = ReadableMsdpVal::read(&mut source.reader()).unwrap();
        assert_eq!(data, ReadableMsdpVal::Ok(original));
    }
}