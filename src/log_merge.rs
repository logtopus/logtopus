use futures::Stream;

pub enum LogStreamError {
    DefaultError,
}

pub type LogStream = Box<Stream<Item = String, Error = LogStreamError>>;

pub struct LogMerge {
    sources: Vec<LogStream>,
}

impl LogMerge {
    pub fn new(sources: Vec<LogStream>) -> LogMerge {
        LogMerge { sources: sources }
    }
}

#[cfg(test)]
mod tests {
    use crate::log_merge::{LogMerge, LogStream, LogStreamError};
    use futures::stream::once;

    #[test]
    fn test_new() {
        let s1: LogStream = Box::new(once(Ok(String::from("s1"))));
        let s2: LogStream = Box::new(once(Ok(String::from("s2"))));
        let sources = vec![s1, s2];
        let merge = LogMerge::new(sources);
        assert!(merge.sources.len() == 2);
    }
}
