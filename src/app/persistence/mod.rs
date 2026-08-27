pub mod receiver;

use std::{
    collections::VecDeque,
    io::{self, BufWriter, Seek, Write as _},
    mem,
    ops::RangeInclusive,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use bytes::BytesMut;
use serde::{Deserialize, Serialize};
use tokio::{
    fs::{self, File},
    io::{AsyncBufReadExt, AsyncReadExt, AsyncSeekExt, BufReader},
};

use crate::app::processing::{ansi::Ansi, text::SystemMessage};

const DEFAULT_SCROLLBACK_SIZE: u32 = 20_000;
const SCROLLBACK_LIMIT_HYSTERESIS: f32 = 1.15;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
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
    offsets: VecDeque<u64>,
    eof: u64,
    pending: Vec<PersistableLine>,
    limit: u32,
}

impl PersistedLinesInternal {
    fn new(path: PathBuf, limit: u32) -> Self {
        Self {
            path,
            offsets: Default::default(),
            eof: 0,
            pending: Default::default(),
            limit,
        }
    }
}

#[derive(Clone)]
pub struct PersistedLines {
    state: Arc<Mutex<PersistedLinesInternal>>,
}

impl PersistedLines {
    fn with_state(state: PersistedLinesInternal) -> io::Result<PersistedLines> {
        Ok(PersistedLines {
            state: Arc::new(Mutex::new(state)),
        })
    }

    pub async fn load(path: PathBuf) -> io::Result<PersistedLines> {
        PersistedLines::load_with_limit(path, DEFAULT_SCROLLBACK_SIZE).await
    }

    pub async fn load_with_limit(path: PathBuf, limit: u32) -> io::Result<PersistedLines> {
        let mut internal = PersistedLinesInternal::new(path, limit);

        let file = match File::open(&internal.path).await {
            Ok(file) => file,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                // New file
                if let Some(parent) = internal.path.parent() {
                    fs::create_dir_all(parent).await?;
                }
                return PersistedLines::with_state(internal);
            }
            Err(err) => return Err(err),
        };
        let reader = BufReader::new(file);

        let mut offset = 0;
        let mut lines = reader.lines();
        while let Some(line) = lines.next_line().await? {
            // NOTE: This *is* byte length, surprisingly
            let len = line.len() + 1; // +1 for \n

            internal.offsets.push_back(offset);
            internal.eof += len as u64;
            offset += len as u64;
        }

        PersistedLines::with_state(internal)
    }

    pub fn persisted_len(&self) -> usize {
        self.state.lock().unwrap().offsets.len()
    }

    #[allow(clippy::await_holding_lock)]
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
            let start = state.offsets[i];
            let end = if i == state.offsets.len() - 1 {
                state.eof
            } else {
                state.offsets[i + 1]
            };
            let len = end - start;

            bytes.clear();
            bytes.resize(len as usize, 0);

            reader.read_exact(bytes.as_mut()).await?;

            let line: PersistableLine = serde_json::from_slice(bytes.as_ref())?;
            lines_read.push(line);
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
        if state.pending.is_empty() {
            state.pending.push(Default::default());
        }
        state
            .pending
            .last_mut()
            .unwrap()
            .parts
            .push(PersistablePart::Ansi(text.to_string()));
    }

    pub fn flush(&self) -> io::Result<()> {
        let mut state = self.state.lock().unwrap();
        if state.pending.is_empty() {
            return Ok(());
        }

        if !state.offsets.is_empty()
            && state.offsets.len() + state.pending.len()
                > (state.limit as f32 * SCROLLBACK_LIMIT_HYSTERESIS) as usize
        {
            self.flush_rotate(&mut state)
        } else {
            let path = state.path.clone();
            self.flush_incremental(&mut state, &path)
        }
    }

    fn flush_incremental<P: AsRef<Path>>(
        &self,
        state: &mut PersistedLinesInternal,
        path: &P,
    ) -> io::Result<()> {
        let file = std::fs::File::options()
            .create(true)
            .append(true)
            .open(path)?;
        let mut writer = BufWriter::new(file);

        let mut pending = vec![];
        mem::swap(&mut state.pending, &mut pending);

        for line in pending.into_iter() {
            let bytes = serde_json::to_vec(&line)?;
            writer.write_all(&bytes)?;
            writer.write_all(b"\n")?;

            let eof = state.eof;
            state.offsets.push_back(eof);
            state.eof += bytes.len() as u64 + 1; // +1 for \n
        }
        writer.flush()?;

        Ok(())
    }

    fn flush_rotate(&self, state: &mut PersistedLinesInternal) -> io::Result<()> {
        let to_rotate = state.limit as usize - state.pending.len();
        let new_0th_idx = state.offsets.len() - to_rotate;
        let start_offset = state.offsets[new_0th_idx];
        let initial_eof = state.eof - start_offset;

        let mut source = std::fs::File::open(&state.path)?;
        source.seek(io::SeekFrom::Start(start_offset))?;

        let pending_path = state.path.with_extension(".pending");
        let mut tmp = std::fs::File::create(&pending_path)?;
        std::io::copy(&mut source, &mut tmp)?;

        state.eof = initial_eof;
        for _ in 0..new_0th_idx {
            state.offsets.pop_front();
        }
        state.offsets.iter_mut().for_each(|v| *v -= start_offset);

        if !state.pending.is_empty() {
            self.flush_incremental(state, &pending_path)?;
        }

        // Swap in the rotated file
        std::fs::rename(pending_path, &state.path)?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::env::temp_dir;

    use crate::app::{
        persistence::{PersistablePart, PersistedLines},
        processing::ansi::Ansi,
    };

    #[tokio::test]
    async fn test_roundtrip() {
        let mut file = temp_dir();
        file.push("test_roundtrip");
        let _ = tokio::fs::remove_file(&file).await;

        let lines = PersistedLines::load(file.clone()).await.unwrap();
        lines.append_to_last_line(&Ansi::from("\x1b[32mhi"));
        lines.push_empty_line();
        lines.append_to_last_line(&Ansi::from("there"));
        lines.push_empty_line();
        lines.flush().unwrap();

        let rt = PersistedLines::load(file).await.unwrap();
        assert_eq!(rt.persisted_len(), 3);
        let loaded = rt.load_line_range(0..=1).await.unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(
            loaded[0].parts[0],
            PersistablePart::Ansi("\x1b[32mhi".to_string())
        );
        assert_eq!(
            loaded[1].parts[0],
            PersistablePart::Ansi("there".to_string())
        );
    }

    #[tokio::test]
    async fn test_rotate() {
        let limit = 10;
        let retainable = 11; // allowed per hysteresis

        let mut file = temp_dir();
        file.push("test_rotate");
        let _ = tokio::fs::remove_file(&file).await;

        let lines = PersistedLines::load_with_limit(file.clone(), limit)
            .await
            .unwrap();
        lines.append_to_last_line(&Ansi::from("#0"));
        for i in 1..retainable {
            lines.push_empty_line();
            lines.append_to_last_line(&Ansi::from(format!("#{i}")));
        }
        lines.flush().unwrap();

        let loaded0 = lines.load_line_range(0..=1).await.unwrap();
        assert_eq!(loaded0.len(), 2);
        assert_eq!(loaded0[0].parts[0], PersistablePart::Ansi("#0".to_string()));
        assert_eq!(loaded0[1].parts[0], PersistablePart::Ansi("#1".to_string()));

        let rt = PersistedLines::load_with_limit(file.clone(), limit)
            .await
            .unwrap();
        assert_eq!(rt.persisted_len(), retainable);

        // Now, append another line, forcing a rotate
        rt.push_empty_line();
        rt.append_to_last_line(&Ansi::from("nth"));
        rt.flush().unwrap();
        assert_eq!(rt.persisted_len() as u32, limit);

        let loaded = rt.load_line_range(0..=1).await.unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].parts[0], PersistablePart::Ansi("#2".to_string()));
        assert_eq!(loaded[1].parts[0], PersistablePart::Ansi("#3".to_string()));

        // After hitting the hysteresis limit, we prune down to
        // the "actual" limit
        let rt2 = PersistedLines::load_with_limit(file, limit).await.unwrap();
        assert_eq!(rt2.persisted_len(), limit as usize);

        let loaded2 = rt.load_line_range(0..=1).await.unwrap();
        assert_eq!(loaded2.len(), 2);
        assert_eq!(loaded2[0].parts[0], PersistablePart::Ansi("#2".to_string()));
        assert_eq!(loaded2[1].parts[0], PersistablePart::Ansi("#3".to_string()));
    }
}
