use std::{collections::HashMap, io};

use async_trait::async_trait;
use bytes::Buf;
use tokio::sync::broadcast::{self, Receiver, Sender};

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

#[derive(Clone, Debug, PartialEq, Eq)]
struct MsdpVar(MsdpName, MsdpVal);

impl Writable for MsdpVar {
    fn write<S: io::Write>(self, stream: &mut S) -> io::Result<()> {
        self.0.write(stream)?;
        self.1.write(stream)
    }
}

impl Readable for MsdpVar {
    fn read<S: io::BufRead>(stream: &mut S) -> io::Result<Self> {
        let name = match ReadableMsdpVal::read(stream)? {
            ReadableMsdpVal::Ok(MsdpVal::String(name)) => MsdpName::from_string(name),
            unexpected => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Expected MSDP_VAR but got {:?}", unexpected),
                ));
            }
        };

        let value = match ReadableMsdpVal::read(stream)? {
            ReadableMsdpVal::Ok(value) => value,
            unexpected => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Expected MSDP_VAL but got {:?}", unexpected),
                ));
            }
        };

        Ok(MsdpVar(name, value))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MsdpName {
    Commands,
    List,
    Report,
    Unreport,
    Other(String),
}

impl MsdpName {
    fn from_string(name: String) -> MsdpName {
        match name.as_str() {
            "COMMANDS" => MsdpName::Commands,
            "LIST" => MsdpName::List,
            "REPORT" => MsdpName::Report,
            "UNREPORT" => MsdpName::Unreport,
            _ => MsdpName::Other(name),
        }
    }

    fn into_string(self) -> String {
        match self {
            MsdpName::Other(s) => s,
            named => format!("{:?}", named).to_uppercase(),
        }
    }
}

impl Writable for MsdpName {
    fn write<S: io::Write>(self, stream: &mut S) -> io::Result<()> {
        stream.write_all(&[MSDP_VAR])?;
        stream.write_all(self.into_string().as_bytes())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MsdpVal {
    String(String),
    Array(Vec<MsdpVal>),
    #[allow(dead_code)]
    FlatArray(Vec<MsdpVal>),
    Table(HashMap<String, MsdpVal>),
}

impl MsdpVal {
    fn is_flat_array(&self) -> bool {
        match self {
            Self::FlatArray(_) => true,
            _ => false,
        }
    }
}

impl Writable for MsdpVal {
    fn write<S: io::Write>(self, stream: &mut S) -> io::Result<()> {
        if !self.is_flat_array() {
            stream.write_all(&[MSDP_VAL])?;
        }

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
                    let buf = stream.fill_buf()?;
                    if buf.get(0) == Some(&MSDP_TABLE_CLOSE) {
                        break;
                    }

                    let var = MsdpVar::read(stream)?;
                    map.insert(var.0.into_string(), var.1);
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
                            return Ok(ReadableMsdpVal::Ok(value));
                        }
                        _ => continue,
                    };
                }

                let string_len = buf.len();
                let content = String::from_utf8_lossy(buf).to_string();
                let value = MsdpVal::String(content);
                stream.consume(string_len);
                return Ok(ReadableMsdpVal::Ok(value));
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum MsdpEvent {
    Reset,
    UpdateVar(String, MsdpVal),
}

pub struct MsdpOptionHandler {
    events: Sender<MsdpEvent>,
}

impl MsdpOptionHandler {
    pub fn new() -> (Self, Receiver<MsdpEvent>) {
        let (sender, receiver) = broadcast::channel(1);
        (MsdpOptionHandler { events: sender }, receiver)
    }
}

impl MsdpOptionHandler {
    fn reset(&self) {
        // Don't worry if nobody's around to receive
        self.events.send(MsdpEvent::Reset).ok();
    }
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
        let var: MsdpVar = MsdpVar::read(&mut data.reader())?;
        log::trace!(target: "telnet", "<< MSDP {:?} {:?}", var.0, var.1);

        match var.0 {
            MsdpName::Commands => {
                // TODO stash capabilities
            }
            _ => {} // ignore
        }

        // TODO Should we store anywhere?
        self.events
            .send(MsdpEvent::UpdateVar(var.0.into_string(), var.1))
            .ok();

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

    #[test]
    fn read_list_var_test() {
        let original = MsdpVar(
            MsdpName::Commands,
            MsdpVal::Array(vec![
                MsdpVal::String("LIST".to_string()),
                MsdpVal::String("REPORT".to_string()),
                MsdpVal::String("RESET".to_string()),
            ]),
        );
        let source = original.clone().into_bytes();

        let data = MsdpVar::read(&mut source.reader()).unwrap();
        assert_eq!(data, original);
    }

    #[test]
    fn read_table_var_test() {
        let original = MsdpVar(
            MsdpName::List,
            MsdpVal::Table(HashMap::from([(
                "LIST".to_string(),
                MsdpVal::String("REPORT".to_string()),
            )])),
        );
        let source = original.clone().into_bytes();

        let data = MsdpVar::read(&mut source.reader()).unwrap();
        assert_eq!(data, original);
    }

    #[ignore]
    #[test]
    fn read_flat_array_test() {
        let original = MsdpVar(
            MsdpName::Report,
            MsdpVal::FlatArray(vec![
                MsdpVal::String("NAME".to_string()),
                MsdpVal::String("LEVEL".to_string()),
            ]),
        );
        let source = original.clone().into_bytes();

        let data = MsdpVar::read(&mut source.reader()).unwrap();
        assert_eq!(data, original);
    }
}
