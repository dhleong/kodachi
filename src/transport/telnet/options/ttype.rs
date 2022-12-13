use std::{env, io};

use async_trait::async_trait;
use bytes::BytesMut;

use crate::transport::telnet::{
    processor::TelnetEvent,
    protocol::{NegotiationType, TelnetOption},
};

use super::{negotiator::OptionsNegotiatorBuilder, DynWriteStream, TelnetOptionHandler};

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
        let name = self.build_name();
        let message = TelnetEvent::Subnegotiate(self.option(), name.freeze());
        log::trace!(target: "telnet", ">> {:?}", message);
        message.write_all(&mut stream).await
    }

    fn build_name(&self) -> BytesMut {
        match self.state {
            State::ClientName => "kodachi".into(),
            State::TermType => env::var("TERM").unwrap_or("".to_string()).as_str().into(),
            State::MttsBitVector => "MTTS 0".into(), // TODO
        }
    }

    fn advance_state(&mut self) {
        self.state = match self.state {
            State::ClientName => State::TermType,
            _ => State::MttsBitVector,
        };
    }
}
