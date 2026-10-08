//! Python bindings for FRGLib, built on SDL3.

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule};

mod window;

use window::{Error, Window as CoreWindow};

fn to_py_err(err: Error) -> PyErr {
    PyRuntimeError::new_err(err.to_string())
}

#[pyclass(unsendable, module = "frglib", name = "Window")]
struct PyWindow {
    inner: Option<CoreWindow>,
}

impl PyWindow {
    fn inner(&self) -> PyResult<&CoreWindow> {
        self.inner
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("window is already closed"))
    }

    fn inner_mut(&mut self) -> PyResult<&mut CoreWindow> {
        self.inner
            .as_mut()
            .ok_or_else(|| PyRuntimeError::new_err("window is already closed"))
    }
}

#[pymethods]
impl PyWindow {
    #[new]
    #[pyo3(signature = (title, width=800, height=600, *, resizable=true, vulkan=false))]
    fn new(title: &str, width: u32, height: u32, resizable: bool, vulkan: bool) -> PyResult<Self> {
        CoreWindow::new(title, width, height, resizable, vulkan)
            .map(|inner| PyWindow { inner: Some(inner) })
            .map_err(to_py_err)
    }

    fn poll(&mut self) -> PyResult<bool> {
        self.inner_mut()?.poll().map_err(to_py_err)
    }

    /// Current drawable size, as `(width, height)`.
    fn size(&self) -> PyResult<(u32, u32)> {
        Ok(self.inner()?.size())
    }

    /// Set the colour `frame` fills the window with, as RGBA bytes.
    fn set_background(&mut self, r: u8, g: u8, b: u8, a: u8) -> PyResult<()> {
        self.inner_mut()?.set_background(r, g, b, a);
        Ok(())
    }

    /// The colour `frame` currently fills the window with, as RGBA bytes.
    #[getter]
    fn background(&self) -> PyResult<(u8, u8, u8, u8)> {
        Ok(self.inner()?.background())
    }

    /// Fill the window with the background colour and present the frame.
    ///
    /// SDL has no window background API, so this has to be called every frame.
    fn frame(&mut self) -> PyResult<()> {
        self.inner_mut()?.frame().map_err(to_py_err)
    }

    /// Vulkan instance extensions required to present to this window.
    fn vulkan_instance_extensions(&self) -> PyResult<Vec<String>> {
        self.inner()?
            .vulkan_instance_extensions()
            .map_err(to_py_err)
    }

    /// Destroy the window. Safe to call more than once.
    fn close(&mut self) {
        self.inner = None;
    }

    fn __enter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    #[pyo3(signature = (exc_type=None, exc_value=None, traceback=None))]
    fn __exit__(
        &mut self,
        exc_type: Option<&Bound<'_, PyAny>>,
        exc_value: Option<&Bound<'_, PyAny>>,
        traceback: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<bool> {
        let _ = (exc_type, exc_value, traceback);
        self.close();
        // Never swallow the exception that got us here.
        Ok(false)
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            Some(inner) => {
                let (w, h) = inner.size();
                format!("<frglib.Window {w}x{h}>")
            }
            None => "<frglib.Window closed>".to_string(),
        }
    }
}

#[pymodule]
fn frglib(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyWindow>()?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
