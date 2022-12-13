use std::{collections::HashMap, io};

use log::trace;
use tokio::io::AsyncWrite;

use crate::transport::telnet::{
    processor::TelnetEvent,
    protocol::{NegotiationType, TelnetOption},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OptionState {
    Accept(NegotiationType),
    Will,
    Do,
}

pub struct OptionsNegotiator {
    options: HashMap<TelnetOption, OptionState>,
}

impl OptionsNegotiator {
    pub async fn negotiate<S: AsyncWrite + Unpin + Send>(
        &mut self,
        negotiation: NegotiationType,
        option: TelnetOption,
        stream: &mut S,
    ) -> io::Result<()> {
        trace!(target: "telnet", "<< {:?} {:?}", negotiation, option);

        if let Some(state) = self.options.get(&option) {
            if state == &OptionState::Accept(negotiation) {
                let (state, response_type) = match negotiation {
                    NegotiationType::Do => (OptionState::Do, NegotiationType::Will),
                    NegotiationType::Will => (OptionState::Will, NegotiationType::Do),
                    _ => panic!("Impossible negotiation {:?} for {:?}", negotiation, option),
                };
                self.options.insert(option, state);

                let response = TelnetEvent::Negotiate(response_type, option);
                trace!(target: "telnet", ">> {:?}", response);
                response.write_all(stream).await?;

                return Ok(());
            }
        }

        let response_type = match negotiation {
            NegotiationType::Do => NegotiationType::Wont,
            NegotiationType::Will => NegotiationType::Dont,
            _ => panic!("Impossible negotiation {:?} for {:?}", negotiation, option),
        };

        let response = TelnetEvent::Negotiate(response_type, option);
        trace!(target: "telnet", ">> {:?}", response);
        response.write_all(stream).await?;

        Ok(())
    }
}

#[derive(Default)]
pub struct OptionsNegotiatorBuilder {
    options: HashMap<TelnetOption, OptionState>,
}

impl OptionsNegotiatorBuilder {
    pub fn build(self) -> OptionsNegotiator {
        OptionsNegotiator {
            options: self.options,
        }
    }

    pub fn accept_do(mut self, option: TelnetOption) -> Self {
        self.options
            .insert(option, OptionState::Accept(NegotiationType::Do));
        self
    }

    #[allow(dead_code)]
    pub fn accept_will(mut self, option: TelnetOption) -> Self {
        self.options
            .insert(option, OptionState::Accept(NegotiationType::Will));
        self
    }
}
