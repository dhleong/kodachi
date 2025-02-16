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
    will_never_get_size: bool,
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

        log::trace!(target: "telnet", ">> (NAWS {width} x {height})");
        TelnetEvent::Subnegotiate(self.option(), response.freeze())
            .write_all(stream)
            .await
    }
}

#[async_trait]
impl TelnetOptionHandler for NawsOptionHandler {
    fn will_answer_negotiation(&self) -> bool {
        true
    }

    fn option(&self) -> TelnetOption {
        TelnetOption::Naws
    }

    fn register(&self, negotiator: OptionsNegotiatorBuilder) -> OptionsNegotiatorBuilder {
        // NOTE: We don't use the default negotiator because it is
        // constructed before we know whether or not we should be
        // enabled.
        negotiator
    }

    async fn notify(
        &mut self,
        notification: &TransportNotification,
        mut stream: DynWriteStream<'_>,
    ) -> io::Result<()> {
        match notification {
            TransportNotification::WindowSizeUnavailable => {
                self.will_never_get_size = true;
            }

            TransportNotification::WindowSize { width, height } => {
                self.set_size(*width, *height, &mut stream).await?;
            }
        }

        Ok(())
    }

    async fn negotiate(
        &mut self,
        negotiation: NegotiationType,
        mut stream: DynWriteStream<'_>,
    ) -> std::io::Result<()> {
        match negotiation {
            NegotiationType::Do => {
                if self.will_never_get_size {
                    TelnetEvent::Negotiate(NegotiationType::Wont, self.option())
                        .write_all(&mut stream)
                        .await
                } else {
                    if !self.enabled {
                        TelnetEvent::Negotiate(NegotiationType::Will, self.option())
                            .write_all(&mut stream)
                            .await?;
                    }

                    self.enabled = true;
                    self.has_sent = false; // Just in case

                    self.try_send(&mut stream).await
                }
            }

            NegotiationType::Dont => {
                self.enabled = false;
                Ok(())
            }

            _ => Ok(()),
        }
    }
}
