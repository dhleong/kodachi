use std::{collections::HashMap, io};

use async_trait::async_trait;
use bytes::Bytes;
use tokio::io::AsyncWrite;

use self::{
    negotiator::{OptionsNegotiator, OptionsNegotiatorBuilder},
    ttype::TermTypeOptionHandler,
};

use super::protocol::{NegotiationType, TelnetOption};

pub mod mccp;
pub mod negotiator;
pub mod ttype;

// NOTE: We need to Box the Stream type in order for TelnetOptionHandler to be object-safe.
pub type DynWriteStream<'a> = Box<&'a mut (dyn AsyncWrite + Unpin + Send)>;

#[async_trait]
pub trait TelnetOptionHandler: Send {
    fn option(&self) -> TelnetOption;
    fn register(&self, negotiator: OptionsNegotiatorBuilder) -> OptionsNegotiatorBuilder;

    async fn negotiate(
        &mut self,
        _negotiation: NegotiationType,
        _stream: DynWriteStream<'_>,
    ) -> io::Result<()> {
        Ok(())
    }

    async fn subnegotiate(&mut self, _data: Bytes, _stream: DynWriteStream<'_>) -> io::Result<()> {
        Ok(())
    }
}

pub struct TelnetOptionsManager {
    negotiator: OptionsNegotiator,
    handlers: HashMap<TelnetOption, Box<dyn TelnetOptionHandler>>,
}

impl Default for TelnetOptionsManager {
    fn default() -> Self {
        let mut negotiator_builder = OptionsNegotiatorBuilder::default();
        let mut handlers: HashMap<TelnetOption, Box<dyn TelnetOptionHandler>> = Default::default();

        // All handlers:
        let all_handlers: Vec<Box<dyn TelnetOptionHandler>> =
            vec![Box::new(TermTypeOptionHandler::default())];

        // Register with the builder
        for handler in all_handlers {
            negotiator_builder = handler.register(negotiator_builder);
            handlers.insert(handler.option(), handler);
        }

        // Build the negotiator, adding in extra options not managed by a Handler
        let negotiator = negotiator_builder.accept_will(TelnetOption::MCCP2).build();

        TelnetOptionsManager {
            negotiator,
            handlers,
        }
    }
}

impl TelnetOptionsManager {
    pub async fn negotiate<S: AsyncWrite + Unpin + Send>(
        &mut self,
        negotiation: NegotiationType,
        option: TelnetOption,
        stream: &mut S,
    ) -> io::Result<()> {
        self.negotiator
            .negotiate(negotiation, option, stream)
            .await?;
        if let Some(handler) = self.handlers.get_mut(&option) {
            let wrapped: Box<&mut (dyn AsyncWrite + Unpin + Send)> = Box::new(stream);
            handler.negotiate(negotiation, wrapped).await?;
        }
        Ok(())
    }

    pub async fn subnegotiate<S: AsyncWrite + Unpin + Send>(
        &mut self,
        option: TelnetOption,
        data: Bytes,
        stream: &mut S,
    ) -> io::Result<()> {
        if self.negotiator.is_accepted(option) {
            if let Some(handler) = self.handlers.get_mut(&option) {
                let wrapped: Box<&mut (dyn AsyncWrite + Unpin + Send)> = Box::new(stream);
                handler.subnegotiate(data, wrapped).await?;
            }
        }
        Ok(())
    }
}
