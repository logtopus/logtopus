use futures::Async::*;
use futures::{Poll, Stream};
use log::*;
use std::fmt;
use std::fmt::Display;

#[derive(Debug)]
pub enum LogMergeError {
    DefaultError,
}

impl Display for LogMergeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Failed stream.")
    }
}

pub type LogStream = Box<Stream<Item = String, Error = LogMergeError>>;

enum SourceState {
    NeedsPoll,
    Delivered(String),
    Finished,
}

pub struct LogMerge {
    sources: Vec<LogStream>,
    source_state: Vec<SourceState>,
}

impl LogMerge {
    pub fn new(sources: Vec<LogStream>) -> LogMerge {
        let mut source_state = Vec::with_capacity(sources.len());
        for i in 0..sources.len() {
            source_state.push(SourceState::NeedsPoll);
        }
        LogMerge {
            sources: sources,
            source_state: source_state,
        }
    }

    fn poll_source(&mut self, s: usize) -> Result<(), LogMergeError> {
        println!("poll source {}", s);
        match self.sources[s].poll() {
            Ok(Ready(Some(line))) => {
                println!("line");
                self.source_state[s] = SourceState::Delivered(line);
            }
            Ok(Ready(None)) => {
                println!("finished");
                self.source_state[s] = SourceState::Finished;
            }
            Ok(NotReady) => {
                println!("not ready");
                self.source_state[s] = SourceState::NeedsPoll;
            }
            Err(e) => {
                error!("Poll failed: {}", e);
                return Err(LogMergeError::DefaultError);
            }
        }
        Ok(())
    }
}

impl Stream for LogMerge {
    type Item = String;
    type Error = LogMergeError;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        println!("poll");
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
        let mut finished = 0;
        for s in 0..self.source_state.len() {
            println!("check delivery source {}", s);
            match &self.source_state[s] {
                SourceState::Delivered(line) => {
                    println!("line");
                    return Ok(Ready(Some(line.to_owned())));
                }
                SourceState::Finished => {
                    println!("finished = {}", finished);
                    finished += 1;
                }
                _ => {}
            }
        }
        println!("{}:{}", finished, self.source_state.len());
        if self.source_state.len() == finished {
            return Ok(Ready(None));
        }
        Ok(NotReady)
        //     if self.cache[s].is_none() {
        //         match self.sources[s].poll() {
        //             Ok(Ready(Some(line))) => {
        //                 // Ok(Ready(Some(line)))
        //                 self.cache[s] = Some(line)
        //             }
        //             Ok(Ready(None)) => {}
        //             Ok(NotReady) => {
        //                 //Ok(NotReady)
        //                 ready = false
        //             }
        //             Err(e) => {
        //                 error!("Poll failed: {}", e);
        //                 return Err(LogMergeError::DefaultError);
        //             }
        //         }
        //     } else {
        //         let line = self.cache[s].to_owned();
        //         self.cache[s] = None;
        //         return Ok(Ready(line));
        //     }
        // }
        // if !ready {
        //     return Ok(NotReady);
        // }
        // Ok(Ready(None))
    }
}

#[cfg(test)]
mod tests {
    use crate::log_merge::{LogMerge, LogMergeError, LogStream};
    use futures::stream::{iter_ok, once};
    use futures::Stream;
    use tokio::runtime::current_thread::Runtime;

    #[test]
    fn test_new() {
        let s1: LogStream = Box::new(once(Ok(String::from("s1"))));
        let s2: LogStream = Box::new(once(Ok(String::from("s2"))));
        let sources = vec![s1, s2];
        let merge = LogMerge::new(sources);
        assert!(merge.sources.len() == 2);
        assert!(merge.source_state.len() == 2);
    }

    #[test]
    fn test_poll_source() {
        assert!(false, "implement")
    }

    #[test]
    fn test_single_stream() {
        let s1: LogStream = Box::new(iter_ok(vec![String::from("s11"), String::from("s12")]));
        let sources = vec![s1];
        let merge = LogMerge::new(sources);
        let mut rt = Runtime::new().unwrap();
        let result = rt.block_on(merge.collect()).unwrap();
        assert_eq!(vec![String::from("s11"), String::from("s12")], result);
    }

    #[test]
    fn test_multiple_streams_same_length() {
        let s1: LogStream = Box::new(iter_ok(vec![String::from("s11"), String::from("s12")]));
        let s2: LogStream = Box::new(iter_ok(vec![String::from("s21"), String::from("s22")]));
        let s3: LogStream = Box::new(iter_ok(vec![String::from("s31"), String::from("s32")]));
        let sources = vec![s1, s2, s3];
        let merge = LogMerge::new(sources);
        let mut rt = Runtime::new().unwrap();
        let result = rt.block_on(merge.collect()).unwrap();
        assert_eq!(
            vec![
                String::from("s11"),
                String::from("s21"),
                String::from("s31"),
                String::from("s21"),
                String::from("s22"),
                String::from("s23")
            ],
            result
        );
    }
}
