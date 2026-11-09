use std::error::Error as StdError;
use std::fmt;

use crate::window::Error as WindowError;

/// What failed while opening a [`WgpuWindow`](super::WgpuWindow) or beginning a
/// frame.
///
/// Kept separate from [`crate::Error`] because the two window flavours fail in
/// different places: this one has no renderer to fail at, and the SDL error of
/// the other one does not describe adapter or device selection.
#[derive(Debug)]
pub enum Error {
    /// SDL initialisation or the video subsystem refused to start.
    Sdl(sdl3::Error),
    /// The SDL window would not build.
    WindowBuild(sdl3::video::WindowBuildError),
    /// Polling events failed inside the shared SDL event pump.
    Window(WindowError),
    /// `wgpu` could not create a surface from the window's native handle.
    Surface(wgpu::CreateSurfaceError),
    /// No adapter supports the surface: no working Vulkan, Metal, DX12 or
    /// GLES 3.0 driver was found.
    NoAdapter(wgpu::RequestAdapterError),
    /// An adapter was found but it would not give us a device.
    Device(wgpu::RequestDeviceError),
    /// The surface reported no usable format. In practice this means the
    /// adapter and the surface do not belong together.
    NoSurfaceFormat,
    /// `get_current_texture` caught a validation error. This is a bug in the
    /// program, not in the window, so it is reported rather than retried.
    SurfaceValidation,
    /// `fontdue` refused to parse the font bytes.
    Font(String),
    /// A draw call named a [`FontId`](super::FontId) this renderer does not
    /// know. A font belongs to the renderer that loaded it.
    UnknownFont,
    /// The glyph atlas ran out of space. Load fewer sizes, or fewer fonts, or
    /// use a renderer with a larger atlas.
    AtlasFull,
    /// A texture was asked for with a zero width or height.
    EmptyTexture,
    /// The RGBA byte slice does not match `width * height * 4`.
    PixelData {
        /// Bytes the given dimensions need.
        expected: usize,
        /// Bytes actually supplied.
        actual: usize,
    },
    /// A Wavefront OBJ model could not be read or parsed.
    Obj(super::ObjError),
    /// A glTF model could not be read or parsed.
    Gltf(super::GltfError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Sdl(err) => write!(f, "SDL error: {err}"),
            Error::WindowBuild(err) => write!(f, "could not create the window: {err}"),
            Error::Window(err) => write!(f, "{err}"),
            Error::Surface(err) => write!(f, "could not create the wgpu surface: {err}"),
            Error::NoAdapter(err) => write!(f, "no suitable GPU adapter: {err}"),
            Error::Device(err) => write!(f, "could not open the GPU device: {err}"),
            Error::NoSurfaceFormat => write!(f, "the GPU surface reported no usable format"),
            Error::SurfaceValidation => {
                write!(f, "wgpu rejected the surface texture: validation error")
            }
            Error::Font(message) => write!(f, "could not parse the font: {message}"),
            Error::UnknownFont => write!(f, "the font does not belong to this renderer"),
            Error::AtlasFull => write!(f, "the glyph atlas is full"),
            Error::EmptyTexture => write!(f, "a texture needs a non-zero width and height"),
            Error::PixelData { expected, actual } => write!(
                f,
                "texture data is {actual} bytes, but the dimensions need {expected}"
            ),
            Error::Obj(err) => write!(f, "OBJ model error: {err}"),
            Error::Gltf(err) => write!(f, "glTF model error: {err}"),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Error::Sdl(err) => Some(err),
            Error::WindowBuild(err) => Some(err),
            Error::Window(err) => Some(err),
            Error::Surface(err) => Some(err),
            Error::NoAdapter(err) => Some(err),
            Error::Device(err) => Some(err),
            Error::Obj(err) => Some(err),
            Error::Gltf(err) => Some(err),
            Error::NoSurfaceFormat
            | Error::SurfaceValidation
            | Error::Font(_)
            | Error::UnknownFont
            | Error::AtlasFull
            | Error::EmptyTexture
            | Error::PixelData { .. } => None,
        }
    }
}

impl From<super::ObjError> for Error {
    fn from(err: super::ObjError) -> Self {
        Error::Obj(err)
    }
}

impl From<super::GltfError> for Error {
    fn from(err: super::GltfError) -> Self {
        Error::Gltf(err)
    }
}

impl From<sdl3::Error> for Error {
    fn from(err: sdl3::Error) -> Self {
        Error::Sdl(err)
    }
}

impl From<sdl3::video::WindowBuildError> for Error {
    fn from(err: sdl3::video::WindowBuildError) -> Self {
        Error::WindowBuild(err)
    }
}

impl From<WindowError> for Error {
    fn from(err: WindowError) -> Self {
        Error::Window(err)
    }
}

impl From<wgpu::CreateSurfaceError> for Error {
    fn from(err: wgpu::CreateSurfaceError) -> Self {
        Error::Surface(err)
    }
}

impl From<wgpu::RequestAdapterError> for Error {
    fn from(err: wgpu::RequestAdapterError) -> Self {
        Error::NoAdapter(err)
    }
}

impl From<wgpu::RequestDeviceError> for Error {
    fn from(err: wgpu::RequestDeviceError) -> Self {
        Error::Device(err)
    }
}
