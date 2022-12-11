use std::{collections::VecDeque, io};

use bytes::{Buf, Bytes, BytesMut};

use super::protocol::IAC;

#[derive(Debug, PartialEq, Eq)]
pub enum TelnetEvent {
    Data(Bytes),
}

#[derive(Clone, Copy)]
enum State {
    Data,
}

impl Default for State {
    fn default() -> Self {
        Self::Data
    }
}

#[derive(Default)]
pub struct TelnetProcessor {
    queue: VecDeque<TelnetEvent>,
    state: State,
}

impl TelnetProcessor {
    pub fn enqueue(&mut self, bytes: &mut BytesMut) -> io::Result<()> {
        let mut i = 0usize;
        while i < bytes.remaining() {
            let byte = (&bytes)[i];
            match (self.state, byte) {
                (State::Data, IAC) => {
                    self.split_data(bytes, i);
                    i = 0;

                    // TODO state transition
                    break;
                }
                (State::Data, _) => {
                    i += 1;
                }
            };
        }

        if i > 0 {
            self.split_data(bytes, i);
        }

        Ok(())
    }

    pub fn pop(&mut self) -> Option<TelnetEvent> {
        self.queue.pop_front()
    }

    fn split_data(&mut self, bytes: &mut BytesMut, at: usize) {
        if at > 0 {
            self.queue
                .push_back(TelnetEvent::Data(bytes.split_to(at).freeze()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_data_test() -> io::Result<()> {
        let bytes = b"For the honor of Grayskull!";

        let mut processor = TelnetProcessor::default();
        processor.enqueue(&mut BytesMut::from(&bytes[..]))?;

        assert_eq!(
            processor.pop(),
            Some(TelnetEvent::Data(Bytes::from(&bytes[..])))
        );

        assert_eq!(processor.pop(), None);
        Ok(())
    }
}
