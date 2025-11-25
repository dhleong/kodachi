use std::io::{self, BufRead, Lines};

use crate::daemon::{protocol::Request, DaemonRequestSource};

pub struct LinesRequestSource<B: BufRead> {
    lines: Lines<B>,
}

impl<B: BufRead> From<B> for LinesRequestSource<B> {
    fn from(value: B) -> Self {
        Self {
            lines: value.lines(),
        }
    }
}

impl<B: BufRead> DaemonRequestSource for LinesRequestSource<B> {
    async fn next(&mut self) -> io::Result<Option<Request>> {
        let Some(line) = self.lines.next() else {
            return Ok(None);
        };
        let raw_json = line?;
        let request: Request = match serde_json::from_str(&raw_json) {
            Ok(request) => request,
            Err(err) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Unable to parse input `{raw_json}`: {err}"),
                ));
            }
        };

        Ok(Some(request))
    }
}
