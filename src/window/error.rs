use std::fmt;

#[derive(Debug, Clone)]
pub struct Error(String);

// println!("{error}")
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl From<sdl3::Error> for Error {
    fn from(err: sdl3::Error) -> Self {
        Error(err.to_string())
    }
}

/// Lets `builder.build()?` convert on its own, instead of every call site
/// wrapping the error by hand.
impl From<sdl3::video::WindowBuildError> for Error {
    fn from(err: sdl3::video::WindowBuildError) -> Self {
        Error(err.to_string())
    }
}
