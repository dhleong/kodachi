use std::{env, io};

use async_trait::async_trait;
use bytes::{BufMut, BytesMut};

use crate::transport::telnet::{
    processor::TelnetEvent,
    protocol::{NegotiationType, TelnetOption},
};

use super::{negotiator::OptionsNegotiatorBuilder, DynWriteStream, TelnetOptionHandler};

const IS: u8 = 0;

enum State {
    ClientName,
    TermType,
    MttsBitVector,
}

pub struct TermTypeOptionHandler {
    state: State,
}

impl Default for TermTypeOptionHandler {
    fn default() -> Self {
        Self {
            state: State::ClientName,
        }
    }
}

#[async_trait]
impl TelnetOptionHandler for TermTypeOptionHandler {
    fn option(&self) -> TelnetOption {
        TelnetOption::TermType
    }

    fn register(&self, negotiator: OptionsNegotiatorBuilder) -> OptionsNegotiatorBuilder {
        negotiator.accept_do(TelnetOption::TermType)
    }

    async fn negotiate(
        &mut self,
        negotiation: NegotiationType,
        stream: DynWriteStream<'_>,
    ) -> io::Result<()> {
        match negotiation {
            NegotiationType::Do => {
                self.respond_with_state(stream).await?;
                self.advance_state();
            }
            NegotiationType::Dont => {
                self.reset();
            }
            _ => {}
        }
        Ok(())
    }
}

impl TermTypeOptionHandler {
    fn reset(&mut self) {
        self.state = State::ClientName;
    }

    async fn respond_with_state(&self, mut stream: DynWriteStream<'_>) -> io::Result<()> {
        let mut response = BytesMut::default();
        response.put_u8(IS);
        self.build_name(&mut response);

        let message = TelnetEvent::Subnegotiate(self.option(), response.freeze());
        log::trace!(target: "telnet", ">> {:?}", message);
        message.write_all(&mut stream).await
    }

    fn build_name(&self, buf: &mut BytesMut) {
        match self.state {
            State::ClientName => buf.put_slice(b"kodachi"),
            State::TermType => buf.put_slice(
                env::var("TERM")
                    .unwrap_or("".to_string())
                    .as_str()
                    .as_bytes(),
            ),
            State::MttsBitVector => buf.put_slice(b"MTTS 0"), // TODO
        }
    }

    fn advance_state(&mut self) {
        self.state = match self.state {
            State::ClientName => State::TermType,
            _ => State::MttsBitVector,
        };
    }
}
