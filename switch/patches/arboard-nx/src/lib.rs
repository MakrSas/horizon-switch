//! arboard's API on the Switch, which has no system clipboard: opening one always fails, so
//! the game keeps copied text in memory.
use std::fmt;

#[derive(Debug)]
pub enum Error {
    ContentNotAvailable,
    ClipboardNotSupported,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the Switch has no system clipboard")
    }
}

impl std::error::Error for Error {}

pub struct Clipboard(());

impl Clipboard {
    pub fn new() -> Result<Self, Error> {
        Err(Error::ClipboardNotSupported)
    }
    pub fn get_text(&mut self) -> Result<String, Error> {
        Err(Error::ContentNotAvailable)
    }
    pub fn set_text<'a, T: Into<std::borrow::Cow<'a, str>>>(&mut self, _text: T) -> Result<(), Error> {
        Err(Error::ClipboardNotSupported)
    }
}
