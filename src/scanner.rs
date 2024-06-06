use tracing::{error, event, info, warn, Level};

use crate::constants::{self};
use core::time;
use std::io::{BufRead, BufReader, Read};

pub enum SplitFnResult {
    Searching,
    SearchingEnd { start: usize },
    Complete { start: usize, end: usize },
}

type SplitFn = dyn Fn(&[u8], usize) -> Result<SplitFnResult, String>;

pub struct Scanner<'a, R: Read> {
    _bufreader: BufReader<R>,
    _split_fn: &'a SplitFn,
}

impl<R: Read> Scanner<'_, R> {
    pub fn from_reader(reader: R, split_fn: &SplitFn) -> Scanner<R> {
        return Scanner {
            _bufreader: BufReader::new(reader),
            _split_fn: split_fn,
        };
    }
}

impl<R: Read> Iterator for Scanner<'_, R> {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut payload_buffer: Vec<u8> = Vec::new();
        let mut start_hint: usize = 0;

        loop {
            let tmp_buffer = self._bufreader.fill_buf().unwrap();
            let tmp_buffer_len = tmp_buffer.len();

            if tmp_buffer_len > 0 {
                payload_buffer.extend(tmp_buffer);
            }

            let split_results = (self._split_fn)(&payload_buffer, start_hint);
            self._bufreader.consume(tmp_buffer_len);

            match split_results {
                Ok(SplitFnResult::Complete { start, end }) => {
                    return Some(payload_buffer[start..end].to_vec());
                }
                Ok(SplitFnResult::SearchingEnd { start }) => {
                    // we have found start but not end of data
                    start_hint = start;
                }
                Ok(SplitFnResult::Searching) => {}
                Err(message) => {
                    // payload_buffer.clear();
                    error!(message);
                }
            }
            std::thread::sleep(time::Duration::from_millis(200));
        }
    }
}
