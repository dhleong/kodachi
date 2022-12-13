use std::io;

use bytes::{Buf, Bytes, BytesMut};
use tokio::io::{AsyncWrite, AsyncWriteExt};

use super::protocol::{
    NegotiationType, TelnetCommand, TelnetOption, DO, DONT, IAC, SB, SE, WILL, WONT,
};

#[derive(Debug, PartialEq, Eq)]
pub enum TelnetEvent {
    Data(Bytes),
    Command(TelnetCommand),
    Negotiate(NegotiationType, TelnetOption),
    Subnegotiate(TelnetOption, Bytes),
}

impl TelnetEvent {
    pub async fn write_all<S: AsyncWrite + Unpin + Send>(self, stream: &mut S) -> io::Result<()> {
        match self {
            TelnetEvent::Data(mut bytes) => stream.write_all_buf(&mut bytes).await,
            TelnetEvent::Command(command) => {
                stream.write_u8(IAC).await?;
                stream.write_u8(command.byte()).await
            }
            TelnetEvent::Negotiate(negotiation, option) => {
                stream.write_u8(IAC).await?;
                stream.write_u8(negotiation.byte()).await?;
                stream.write_u8(option.byte()).await
            }
            TelnetEvent::Subnegotiate(option, mut bytes) => {
                stream.write_u8(IAC).await?;
                stream.write_u8(SB).await?;

                stream.write_u8(option.byte()).await?;
                stream.write_all_buf(&mut bytes).await?;

                stream.write_u8(IAC).await?;
                stream.write_u8(SE).await
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Data,
    InterpretAsCommand,
    Negotiate(NegotiationType),
    Subnegotiate,
    SubnegotiateIac,
}

impl Default for State {
    fn default() -> Self {
        Self::Data
    }
}

#[derive(Default)]
pub struct TelnetProcessor {
    state: State,
}

impl TelnetProcessor {
    pub fn process_one(&mut self, bytes: &mut BytesMut) -> io::Result<Option<TelnetEvent>> {
        let mut i = 0usize;
        while i < bytes.remaining() {
            let byte = bytes[i];
            match (self.state, byte) {
                (State::Data, IAC) => {
                    if let Some(data) = self.split_data(bytes, i) {
                        return Ok(Some(data));
                    }

                    self.state = State::InterpretAsCommand;
                    bytes.get_u8();
                    i = 0;
                }

                (State::Data, _) => {
                    i += 1;
                }

                //
                // IAC handling
                (State::InterpretAsCommand, IAC) => {
                    // Consume the literal byte as data
                    self.state = State::Data;
                    i += 1;
                }

                (State::InterpretAsCommand, WILL | WONT | DO | DONT) => {
                    self.state = State::Negotiate(NegotiationType::from_byte(bytes.get_u8()));
                }

                (State::InterpretAsCommand, SB) => {
                    self.state = State::Subnegotiate;
                    bytes.get_u8();
                }

                (State::InterpretAsCommand, command) => {
                    self.state = State::Data;
                    bytes.get_u8();
                    return Ok(Some(TelnetEvent::Command(TelnetCommand::from_byte(
                        command,
                    ))));
                }

                //
                // Option Negotiation
                (State::Negotiate(negotiation), _) => {
                    self.state = State::Data;
                    return Ok(Some(TelnetEvent::Negotiate(
                        negotiation,
                        TelnetOption::from_byte(bytes.get_u8()),
                    )));
                }

                //
                // Subnegotiation
                (State::Subnegotiate, IAC) => {
                    self.state = State::SubnegotiateIac;
                    i += 1;
                }
                (State::Subnegotiate, _) => {
                    i += 1;
                }

                (State::SubnegotiateIac, IAC) => {
                    // Literal IAC byte; chop off the bytes before...
                    let mut after = bytes.split_off(i);

                    // Consume the extra IAC
                    after.get_u8();

                    // Then shove the rest of the bytes back on the end
                    bytes.unsplit(after);
                }
                (State::SubnegotiateIac, SE) => {
                    self.state = State::Data;
                    let data_end = i.checked_sub(1);
                    let mut data = bytes.split_to(data_end.unwrap_or(0)).freeze();

                    // Consume SE and IAC
                    bytes.get_u8();
                    if data_end.is_some() {
                        bytes.get_u8();
                    }

                    let option_byte = data.get_u8();
                    let option = TelnetOption::from_byte(option_byte);
                    return Ok(Some(TelnetEvent::Subnegotiate(option, data)));
                }
                (State::SubnegotiateIac, _) => {
                    // Unexpected byte; I guess just consume it
                    self.state = State::Subnegotiate;
                    i += 1;
                }
            };
        }

        if i > 0 && self.state == State::Data {
            return Ok(self.split_data(bytes, i));
        }

        Ok(None)
    }

    fn split_data(&mut self, bytes: &mut BytesMut, at: usize) -> Option<TelnetEvent> {
        if at > 0 {
            Some(TelnetEvent::Data(bytes.split_to(at).freeze()))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nop_test() -> io::Result<()> {
        let mut processor = TelnetProcessor::default();
        let mut buffer = BytesMut::new();
        assert_eq!(processor.process_one(&mut buffer)?, None);
        Ok(())
    }

    #[test]
    fn simple_data_test() -> io::Result<()> {
        let bytes = b"For the honor of Grayskull!";

        let mut processor = TelnetProcessor::default();
        let mut buffer = BytesMut::from(&bytes[..]);

        assert_eq!(
            processor.process_one(&mut buffer)?,
            Some(TelnetEvent::Data(Bytes::from(&bytes[..])))
        );

        assert_eq!(processor.process_one(&mut buffer)?, None);
        Ok(())
    }

    #[test]
    fn literal_iac_data() -> io::Result<()> {
        let bytes = b"For the \xFF\xFFhonor\xFF\xFF of Grayskull!";

        let mut processor = TelnetProcessor::default();
        let mut buffer = BytesMut::from(&bytes[..]);

        assert_eq!(
            processor.process_one(&mut buffer)?,
            Some(TelnetEvent::Data(Bytes::from(&b"For the "[..])))
        );
        assert_eq!(
            processor.process_one(&mut buffer)?,
            Some(TelnetEvent::Data(Bytes::from(&b"\xFFhonor"[..])))
        );
        assert_eq!(
            processor.process_one(&mut buffer)?,
            Some(TelnetEvent::Data(Bytes::from(&b"\xFF of Grayskull!"[..])))
        );

        assert_eq!(processor.process_one(&mut buffer)?, None);
        Ok(())
    }

    #[test]
    fn negotiations_test() -> io::Result<()> {
        let bytes = b"For\xFF\xFB\x18the";

        let mut processor = TelnetProcessor::default();
        let mut buffer = BytesMut::from(&bytes[..]);

        assert_eq!(
            processor.process_one(&mut buffer)?,
            Some(TelnetEvent::Data(Bytes::from(&b"For"[..])))
        );
        assert_eq!(
            processor.process_one(&mut buffer)?,
            Some(TelnetEvent::Negotiate(
                NegotiationType::Will,
                TelnetOption::TermType
            ))
        );
        assert_eq!(
            processor.process_one(&mut buffer)?,
            Some(TelnetEvent::Data(Bytes::from(&b"the"[..])))
        );

        assert_eq!(processor.process_one(&mut buffer)?, None);
        Ok(())
    }

    #[test]
    fn subnegotiations_test() -> io::Result<()> {
        let bytes = b"\xFF\xFA\x45\x01VARNAME\x02THE VALUE\xFF\xF0";

        let mut processor = TelnetProcessor::default();
        let mut buffer = BytesMut::from(&bytes[..]);

        assert_eq!(
            processor.process_one(&mut buffer)?,
            Some(TelnetEvent::Subnegotiate(
                TelnetOption::MSDP,
                Bytes::from(&b"\x01VARNAME\x02THE VALUE"[..])
            ))
        );

        assert_eq!(processor.process_one(&mut buffer)?, None);
        Ok(())
    }

    #[test]
    fn subnegotiations_literal_iac_test() -> io::Result<()> {
        let bytes = b"\xFF\xFA\x45\x01VARNAME\x02THE\xFF\xFFVALUE\xFF\xF0";

        let mut processor = TelnetProcessor::default();
        let mut buffer = BytesMut::from(&bytes[..]);

        assert_eq!(
            processor.process_one(&mut buffer)?,
            Some(TelnetEvent::Subnegotiate(
                TelnetOption::MSDP,
                Bytes::from(&b"\x01VARNAME\x02THE\xFFVALUE"[..])
            ))
        );

        assert_eq!(processor.process_one(&mut buffer)?, None);
        Ok(())
    }

    #[test]
    fn incomplete_subnegotiations_test() -> io::Result<()> {
        let bytes = b"\xFF\xFA\x45\x01VARNAME\x02";

        let mut processor = TelnetProcessor::default();
        let mut buffer = BytesMut::from(&bytes[..]);

        assert_eq!(processor.process_one(&mut buffer)?, None);
        Ok(())
    }
}
