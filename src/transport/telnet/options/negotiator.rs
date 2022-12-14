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
            match (negotiation, state) {
                (_, &OptionState::Accept(negotiation)) => {
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

                (NegotiationType::Do, &OptionState::Do)
                | (NegotiationType::Will, &OptionState::Will) => {
                    // Already accepted; this is a nop
                    return Ok(());
                }

                _ => {} // Ignore and fall through below:
            }
        }

        let response_type = match negotiation {
            NegotiationType::Do => Some(NegotiationType::Wont),
            NegotiationType::Will => Some(NegotiationType::Dont),
            NegotiationType::Dont => {
                if self.options.get(&option) == Some(&OptionState::Do) {
                    self.options
                        .insert(option, OptionState::Accept(NegotiationType::Do));
                }
                Some(NegotiationType::Wont)
            }
            NegotiationType::Wont => {
                if self.options.get(&option) == Some(&OptionState::Will) {
                    self.options
                        .insert(option, OptionState::Accept(NegotiationType::Will));
                }
                None
            }
        };

        if let Some(response_type) = response_type {
            let response = TelnetEvent::Negotiate(response_type, option);
            trace!(target: "telnet", ">> {:?}", response);
            response.write_all(stream).await?;
        }

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

#[cfg(test)]
mod tests {
    use std::task::Poll;

    use bytes::BytesMut;

    use super::*;

    struct TestStream {
        sent: BytesMut,
    }

    impl TestStream {
        pub fn new() -> Self {
            Self {
                sent: BytesMut::default(),
            }
        }
    }

    impl AsyncWrite for TestStream {
        fn poll_write(
            mut self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
            buf: &[u8],
        ) -> std::task::Poll<Result<usize, io::Error>> {
            self.sent.extend_from_slice(buf);
            Poll::Ready(Ok(buf.len()))
        }

        fn poll_flush(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Result<(), io::Error>> {
            todo!()
        }

        fn poll_shutdown(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Result<(), io::Error>> {
            todo!()
        }
    }

    #[tokio::test]
    async fn do_after_do_test() -> io::Result<()> {
        let mut handler = OptionsNegotiatorBuilder::default()
            .accept_do(TelnetOption::TermType)
            .build();

        let mut stream = TestStream::new();

        handler
            .negotiate(NegotiationType::Do, TelnetOption::TermType, &mut stream)
            .await?;

        handler
            .negotiate(NegotiationType::Do, TelnetOption::TermType, &mut stream)
            .await?;

        let mut expected_stream = TestStream::new();
        TelnetEvent::Negotiate(NegotiationType::Will, TelnetOption::TermType)
            .write_all(&mut expected_stream)
            .await?;
        assert_eq!(stream.sent, expected_stream.sent);

        Ok(())
    }

    #[tokio::test]
    async fn dont_after_do_test() -> io::Result<()> {
        let mut handler = OptionsNegotiatorBuilder::default()
            .accept_do(TelnetOption::TermType)
            .build();

        let mut stream = TestStream::new();

        handler
            .negotiate(NegotiationType::Do, TelnetOption::TermType, &mut stream)
            .await?;

        handler
            .negotiate(NegotiationType::Dont, TelnetOption::TermType, &mut stream)
            .await?;

        let mut expected_stream = TestStream::new();
        TelnetEvent::Negotiate(NegotiationType::Will, TelnetOption::TermType)
            .write_all(&mut expected_stream)
            .await?;
        TelnetEvent::Negotiate(NegotiationType::Wont, TelnetOption::TermType)
            .write_all(&mut expected_stream)
            .await?;
        assert_eq!(stream.sent, expected_stream.sent);

        // If the server changes their mind, our state should be ready for that
        expected_stream.sent.clear();
        stream.sent.clear();
        handler
            .negotiate(NegotiationType::Do, TelnetOption::TermType, &mut stream)
            .await?;

        TelnetEvent::Negotiate(NegotiationType::Will, TelnetOption::TermType)
            .write_all(&mut expected_stream)
            .await?;
        assert_eq!(stream.sent, expected_stream.sent);

        Ok(())
    }
}
