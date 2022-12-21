use std::{collections::HashMap, io};

use async_trait::async_trait;

use crate::{
    net::writable::{ObjectWriteStream, Writable},
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

struct MsdpVar(String, MsdpVal);

impl Writable for MsdpVar {
    fn write<S: io::Write>(self, stream: &mut S) -> io::Result<()> {
        stream.write_all(&[MSDP_VAR])?;
        stream.write_all(&self.0.as_bytes())?;
        self.1.write(stream)
    }
}

pub enum MsdpVal {
    String(String),
    Array(Vec<MsdpVal>),
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
            MsdpVal::Table(items) => {
                stream.write_all(&[MSDP_TABLE_OPEN])?;
                for (key, val) in items {
                    MsdpVar(key, val).write(stream)?;
                }
                stream.write_all(&[MSDP_TABLE_CLOSE])
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
                let to_send = MsdpVar("LIST".to_string(), MsdpVal::String("COMMANDS".to_string()));
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
