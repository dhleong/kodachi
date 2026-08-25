pub mod receiver;

use std::{
    io::{self, BufWriter, Write as _},
    mem,
    ops::RangeInclusive,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use bytes::BytesMut;
use serde::{Deserialize, Serialize};
use tokio::{
    fs::File,
    io::{AsyncBufReadExt, AsyncReadExt, AsyncSeekExt, BufReader},
};

use crate::app::processing::{ansi::Ansi, text::SystemMessage};

#[derive(Serialize, Deserialize)]
pub enum PersistablePart {
    Ansi(String),
    SystemMessage(SystemMessage),
}

#[derive(Default, Serialize, Deserialize)]
pub struct PersistableLine {
    parts: Vec<PersistablePart>,
}

#[derive(Default)]
struct PersistedLinesInternal {
    path: PathBuf,
    offsets: Vec<u64>,
    eof: u64,
    pending: Vec<PersistableLine>,
}

impl PersistedLinesInternal {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            offsets: vec![],
            eof: 0,
            pending: Default::default(),
        }
    }
}

#[derive(Clone)]
pub struct PersistedLines {
    state: Arc<Mutex<PersistedLinesInternal>>,
}

impl PersistedLines {
    pub async fn load(path: PathBuf) -> io::Result<PersistedLines> {
        let mut internal = PersistedLinesInternal::new(path);

        let file = File::open(&internal.path).await?;
        let reader = BufReader::new(file);

        let mut offset = 0;
        let mut lines = reader.lines();
        while let Some(line) = lines.next_line().await? {
            // NOTE: This *is* byte length, surprisingly
            let len = line.len();

            internal.offsets.push(offset);
            internal.eof += len as u64;
            offset += len as u64;
        }

        Ok(PersistedLines {
            state: Arc::new(Mutex::new(internal)),
        })
    }

    pub fn len(&self) -> usize {
        self.state.lock().unwrap().offsets.len()
    }

    pub async fn load_line_range(
        &self,
        range: RangeInclusive<usize>,
    ) -> io::Result<Vec<PersistableLine>> {
        if range.is_empty() {
            return Ok(vec![]);
        }

        let state = self.state.lock().unwrap();
        if *range.start() >= state.offsets.len() || *range.end() >= state.offsets.len() {
            return Err(io::ErrorKind::InvalidInput.into());
        }

        let abs_start = state.offsets[*range.start()];

        let mut file = tokio::fs::File::open(&state.path).await?;
        file.seek(io::SeekFrom::Start(abs_start)).await?;

        let mut reader = BufReader::new(file);
        let mut bytes = BytesMut::new();
        let mut lines_read = Vec::with_capacity(range.end() - range.start());
        for i in range {
            let start = state.offsets[0];
            let end = if i == state.offsets.len() - 1 {
                state.eof
            } else {
                state.offsets[i + 1]
            };
            let len = end - start - 1; // ignore the newline

            bytes.clear();
            bytes.resize(len as usize, 0);

            reader.read_exact(bytes.as_mut()).await;

            let reader = flexbuffers::Reader::get_root(bytes.as_ref()).unwrap();
            lines_read.push(PersistableLine::deserialize(reader).unwrap());
        }

        Ok(lines_read)
    }

    pub fn push_empty_line(&self) {
        let mut state = self.state.lock().unwrap();
        state.pending.push(Default::default());
    }

    pub fn clear_last_line(&self) {
        let mut state = self.state.lock().unwrap();
        if let Some(last) = state.pending.last_mut() {
            last.parts.clear();
        }
    }

    pub fn append_to_last_line(&self, text: &Ansi) {
        let mut state = self.state.lock().unwrap();
        if let Some(last) = state.pending.last_mut() {
            last.parts.push(PersistablePart::Ansi(text.to_string()));
        }
    }

    pub fn flush(&self) -> io::Result<()> {
        let mut state = self.state.lock().unwrap();
        if state.pending.is_empty() {
            return Ok(());
        }

        let file = std::fs::File::options().append(true).open(&state.path)?;
        let mut writer = BufWriter::new(file);
        let mut serializer = flexbuffers::FlexbufferSerializer::new();

        let mut pending = vec![];
        mem::swap(&mut state.pending, &mut pending);

        for line in pending.into_iter() {
            serializer.reset();
            line.serialize(&mut serializer).unwrap();
            let bytes = serializer.view();
            writer.write_all(bytes)?;
            writer.write_all(b"\n")?;

            let eof = state.eof;
            state.offsets.push(eof);
            state.eof += bytes.len() as u64 + 1; // +1 for \n
        }
        writer.flush()?;

        Ok(())
    }
}
