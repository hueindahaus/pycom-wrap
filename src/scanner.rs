use crate::constants::{self};
use core::time;
use std::io::{BufRead, BufReader, Read};

pub struct Scanner<'a, R: Read> {
    _bufreader: BufReader<R>,
    _split_fn: &'a dyn Fn(&[u8]) -> Result<(usize, &[u8], bool), String>,
}

impl<R: Read> Scanner<'_, R> {
    pub fn from_reader(
        reader: R,
        split_fn: &dyn Fn(&[u8]) -> Result<(usize, &[u8], bool), String>,
    ) -> Scanner<R> {
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
        loop {
            let mut string_buffer = String::new();
            let _ = self._bufreader.read_line(&mut string_buffer);

            payload_buffer.append(&mut string_buffer.as_bytes().to_vec());

            let (_, data, complete) = (self._split_fn)(&payload_buffer).unwrap();

            if complete {
                return Some(data.to_vec());
            }
            std::thread::sleep(time::Duration::from_millis(100));
        }
    }
}
