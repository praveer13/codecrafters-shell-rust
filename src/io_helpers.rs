use std::fs::{File, OpenOptions};
use std::io::{self, Write};

use crate::parser::RedirectType;

pub fn get_write_output(redirect_filename: &str, redirect_type: RedirectType) -> io::Result<File> {
    match redirect_type {
        RedirectType::APPEND => OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(redirect_filename),
        RedirectType::CREATE => OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(redirect_filename),
    }
}

pub enum OutputSink<'a> {
    Stdout(io::StdoutLock<'a>),
    Stderr(io::StderrLock<'a>),
    File(File),
}

impl<'a> Write for OutputSink<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            OutputSink::Stdout(handle) => handle.write(buf),
            OutputSink::Stderr(handle) => handle.write(buf),
            OutputSink::File(file) => file.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            OutputSink::Stdout(handle) => handle.flush(),
            OutputSink::Stderr(handle) => handle.flush(),
            OutputSink::File(file) => file.flush(),
        }
    }
}
