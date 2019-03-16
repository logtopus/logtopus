use crate::tentacle::LogLine;
use futures::Async::*;
use futures::{Poll, Stream};
use std::fmt;
use std::fmt::Display;
use std::vec::Vec;

#[derive(Debug)]
pub enum LogStreamError {
    DefaultError,
}

impl Display for LogStreamError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Failed stream.")
    }
}

pub type LogStream = Box<Stream<Item = LogLine, Error = LogStreamError>>;

#[derive(PartialEq)]
enum SourceState {
    NeedsPoll,
    Delivered,
    Finished,
}

#[derive(Debug)]
struct BufferEntry {
    log_line: LogLine,
    source_idx: usize,
}

pub struct LogMerge {
    running_sources: usize,
    sources: Vec<LogStream>,
    source_state: Vec<SourceState>,
    buffer: Vec<BufferEntry>,
}

impl LogMerge {
    pub fn new(sources: Vec<LogStream>) -> LogMerge {
        let num_sources = sources.len();
        let mut source_state = Vec::with_capacity(sources.len());
        for _ in 0..sources.len() {
            source_state.push(SourceState::NeedsPoll);
        }
        LogMerge {
            running_sources: num_sources,
            sources: sources,
            source_state: source_state,
            buffer: Vec::with_capacity(num_sources),
        }
    }

    fn next_entry(&mut self) -> BufferEntry {
        // TODO: better error handling, remove_item -> rust nightly / 2019-02-20
        let e = self.buffer.remove(0);
        return e;
    }

    fn insert_into_buffer(&mut self, log_line: LogLine, source_idx: usize) {
        let line = BufferEntry {
            log_line,
            source_idx,
        };
        let buffer_size = self.buffer.len();
        let mut insert_at = 0;
        for idx in 0..buffer_size {
            if line.log_line.timestamp < self.buffer[idx].log_line.timestamp {
                break;
            }
            insert_at += 1;
        }
        self.buffer.insert(insert_at, line);
    }

    fn poll_source(&mut self, source_idx: usize) -> Result<(), LogStreamError> {
        match self.sources[source_idx].poll() {
            Ok(Ready(Some(line))) => {
                self.insert_into_buffer(line, source_idx);
                self.source_state[source_idx] = SourceState::Delivered;
            }
            Ok(Ready(None)) => {
                self.source_state[source_idx] = SourceState::Finished;
                self.running_sources -= 1;
            }
            Ok(NotReady) => {
                self.source_state[source_idx] = SourceState::NeedsPoll;
            }
            Err(_) => {
                return Err(LogStreamError::DefaultError);
            }
        }
        Ok(())
    }
}

impl Stream for LogMerge {
    type Item = LogLine;
    type Error = LogStreamError;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        for s in 0..self.source_state.len() {
            match self.source_state[s] {
                SourceState::NeedsPoll => {
                    if let Err(err) = self.poll_source(s) {
                        return Err(err);
                    }
                }
                _ => {}
            }
        }
        if self.running_sources == 0 {
            Ok(Ready(None))
        } else if self.running_sources == self.buffer.len() {
            let entry = self.next_entry();
            self.source_state[entry.source_idx] = SourceState::NeedsPoll;
            Ok(Ready(Some(entry.log_line)))
        } else {
            Ok(NotReady)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::log_merge::{LogMerge, LogStream};
    use crate::tentacle::LogLine;
    use futures::stream::{empty, iter_ok, once};
    use futures::Stream;
    use tokio::runtime::current_thread::Runtime;

    fn line_at(timestamp: i64, line: &str) -> LogLine {
        LogLine {
            timestamp: timestamp,
            message: line.to_string(),
            loglevel: None,
            id: String::from("system-syslog"),
            source: String::from("node1"),
        }
    }

    #[test]
    fn test_new() {
        let s1: LogStream = Box::new(once(Ok(line_at(0, "s1"))));
        let s2: LogStream = Box::new(once(Ok(line_at(1, "s2"))));
        let sources = vec![s1, s2];
        let merge = LogMerge::new(sources);
        assert!(merge.sources.len() == 2);
        assert!(merge.source_state.len() == 2);
    }

    #[test]
    fn empty_streams() {
        let s1: LogStream = Box::new(empty());
        let sources = vec![s1];
        let merge = LogMerge::new(sources);
        let mut rt = Runtime::new().unwrap();
        let result = rt.block_on(merge.collect()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_single_stream() {
        let l1 = line_at(0, "s1");
        let l2 = line_at(1, "s1");
        let s1: LogStream = Box::new(iter_ok(vec![l1.clone(), l2.clone()]));
        let sources = vec![s1];
        let merge = LogMerge::new(sources);
        let mut rt = Runtime::new().unwrap();
        let result = rt.block_on(merge.collect()).unwrap();
        assert_eq!(vec![l1, l2], result);
    }

    #[test]
    fn test_multiple_streams() {
        let l11 = line_at(100, "s11");
        let l12 = line_at(300, "s12");
        let l13 = line_at(520, "s13");
        let l21 = line_at(90, "s21");
        let l22 = line_at(430, "s22");
        let l31 = line_at(120, "s31");
        let l32 = line_at(120, "s32");
        let l33 = line_at(320, "s33");
        let l34 = line_at(520, "s34");
        let s1: LogStream = Box::new(iter_ok(vec![l11.clone(), l12.clone(), l13.clone()]));
        let s2: LogStream = Box::new(iter_ok(vec![l21.clone(), l22.clone()]));
        let s3: LogStream = Box::new(iter_ok(vec![
            l31.clone(),
            l32.clone(),
            l33.clone(),
            l34.clone(),
        ]));
        let sources = vec![s1, s2, s3];
        let merge = LogMerge::new(sources);
        let mut rt = Runtime::new().unwrap();
        let result = rt.block_on(merge.collect()).unwrap();
        assert_eq!(vec![l21, l11, l31, l32, l12, l33, l22, l13, l34], result);
    }
}
