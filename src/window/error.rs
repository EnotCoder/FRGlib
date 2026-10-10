use std::error::Error as StdError;
use std::fmt;

/// What failed inside the library.
///
/// The SDL error type is kept instead of being flattened into a string, so the
/// caller can tell an SDL failure from a window that would not build, and can
/// reach SDL's own message through [`StdError::source`].
#[derive(Debug)]
pub enum Error {
    /// SDL itself refused or failed: initialisation, the video subsystem, or a
    /// query made against an existing window.
    Sdl(sdl3::Error),
    /// The window would not build. Carries the reason, e.g. a title containing
    /// a nul byte or a size that does not fit SDL's integer range.
    WindowBuild(sdl3::video::WindowBuildError),
    /// A renderer would not attach to the window.
    Renderer(sdl3::IntegerOrSdlError),
    /// Presenting the frame failed. SDL reports this as a plain flag rather
    /// than a dedicated error type, so it borrows [`Error::Sdl`]'s payload.
    Present(sdl3::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Sdl(err) => write!(f, "SDL error: {err}"),
            Error::WindowBuild(err) => write!(f, "could not create the window: {err}"),
            Error::Renderer(err) => write!(f, "could not create the renderer: {err}"),
            Error::Present(err) => write!(f, "could not present the frame: {err}"),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Error::Sdl(err) | Error::Present(err) => Some(err),
            Error::WindowBuild(err) => Some(err),
            Error::Renderer(err) => Some(err),
        }
    }
}

impl From<sdl3::Error> for Error {
    fn from(err: sdl3::Error) -> Self {
        Error::Sdl(err)
    }
}

/// Lets `builder.build()?` convert on its own, instead of every call site
/// wrapping the error by hand.
impl From<sdl3::video::WindowBuildError> for Error {
    fn from(err: sdl3::video::WindowBuildError) -> Self {
        Error::WindowBuild(err)
    }
}

/// Same for `sdl3::render::create_renderer`.
impl From<sdl3::IntegerOrSdlError> for Error {
    fn from(err: sdl3::IntegerOrSdlError) -> Self {
        Error::Renderer(err)
    }
}
