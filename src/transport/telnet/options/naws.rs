use std::io;

use async_trait::async_trait;
use bytes::{BufMut, BytesMut};
use tokio::io::AsyncWrite;

use crate::transport::{
    telnet::{
        processor::TelnetEvent,
        protocol::{NegotiationType, TelnetOption},
    },
    TransportNotification,
};

use super::{negotiator::OptionsNegotiatorBuilder, DynWriteStream, TelnetOptionHandler};

#[derive(Default)]
pub struct NawsOptionHandler {
    width: u16,
    height: u16,
    enabled: bool,
    has_sent: bool,
}

impl NawsOptionHandler {
    async fn set_size<S: AsyncWrite + Unpin + Send>(
        &mut self,
        width: u16,
        height: u16,
        stream: &mut S,
    ) -> io::Result<()> {
        if self.width == width && self.height == height && self.has_sent {
            // Nop!
            return Ok(());
        }

        self.width = width;
        self.height = height;

        if self.enabled {
            self.try_send(stream).await
        } else {
            Ok(())
        }
    }

    async fn try_send<S: AsyncWrite + Unpin + Send>(&mut self, stream: &mut S) -> io::Result<()> {
        self.has_sent = true;

        let width = self.width;
        let height = self.height;

        let mut response = BytesMut::default();
        response.put_u16(width);
        response.put_u16(height);

        let message = TelnetEvent::Subnegotiate(self.option(), response.freeze());
        log::trace!(target: "telnet", ">> {message:?} (NAWS {width} x {height})");
        message.write_all(stream).await
    }
}

#[async_trait]
impl TelnetOptionHandler for NawsOptionHandler {
    fn option(&self) -> TelnetOption {
        TelnetOption::Naws
    }

    fn register(&self, negotiator: OptionsNegotiatorBuilder) -> OptionsNegotiatorBuilder {
        negotiator.accept_do(TelnetOption::Naws)
    }

    async fn notify(
        &mut self,
        notification: &TransportNotification,
        mut stream: DynWriteStream<'_>,
    ) -> io::Result<()> {
        #[allow(irrefutable_let_patterns)]
        let TransportNotification::WindowSize { width, height } = notification
        else {
            return Ok(());
        };

        self.set_size(*width, *height, &mut stream).await
    }

    async fn negotiate(
        &mut self,
        negotiation: NegotiationType,
        mut stream: DynWriteStream<'_>,
    ) -> std::io::Result<()> {
        match negotiation {
            NegotiationType::Do => {
                self.enabled = true;
                self.has_sent = false; // Just in case
                self.try_send(&mut stream).await
            }

            NegotiationType::Dont => {
                self.enabled = false;
                Ok(())
            }

            _ => Ok(()),
        }
    }
}
