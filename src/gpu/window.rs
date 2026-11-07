//! The window, the GPU handles behind it, and one frame.
//!
//! SDL3 is used for what it is good at -- creating the platform window,
//! pumping events, and handing over the native handle -- and then steps aside.
//! The swapchain, the depth buffer and the command encoder belong to `wgpu`.

use sdl3::video::Window as SdlWindow;
use sdl3::Sdl;

use crate::event::Event;
use crate::graphics::Color;
use crate::window::pump::with_pump;

use super::error::Error;
use super::WgpuWindowBuilder;

use super::DEPTH_FORMAT;

/// A window whose frames are drawn with `wgpu`.
///
/// Opened with [`WgpuWindow::builder`]. The window is `!Send` for the same
/// reason [`crate::Window`] is: SDL requires it to live on the thread that
/// initialised the video subsystem, normally the main thread.
///
/// A frame is a three-step loop:
///
/// ```no_run
/// # use frglib::{Color, Event};
/// # use frglib::gpu::{Camera3D, Renderer3D, WgpuWindow};
/// # use frglib::gpu::glam::Vec3;
/// # fn main() -> Result<(), frglib::gpu::Error> {
/// # let mut window = WgpuWindow::builder("demo").build()?;
/// # let mut renderer = Renderer3D::new(window.gpu());
/// # let camera = Camera3D::perspective(60.0, 0.1, 100.0);
/// loop {
///     for event in window.poll()? {
///         if matches!(event, Event::Quit | Event::CloseRequested) {
///             return Ok(());
///         }
///     }
///
///     let Some(mut frame) = window.begin_frame()? else { continue };
///     renderer.begin(&camera);
///     renderer.draw_cube(Vec3::ZERO, Vec3::ONE, Color::RED);
///     renderer.finish(&mut frame);
///     window.present(frame)?;
/// }
/// # }
/// ```
///
/// `begin_frame` returns `Ok(None)` when the window is minimised or the
/// swapchain is not ready. Skipping that frame is normal, not an error.
pub struct WgpuWindow {
    // Declaration order is drop order. A surface is tied to the native handle
    // of its window, so it must go first; the SDL window in turn must outlive
    // `_sdl`'s video subsystem.
    surface: wgpu::Surface<'static>,
    window: SdlWindow,
    depth: wgpu::TextureView,
    gpu: Gpu,
    background: Color,
    _sdl: Sdl,
}

impl WgpuWindow {
    /// Starts building a window. See [`WgpuWindowBuilder`].
    pub fn builder(title: impl Into<String>) -> WgpuWindowBuilder {
        WgpuWindowBuilder::new(title)
    }

    /// Opens the window, creates the surface, and picks an adapter and device.
    pub(crate) fn create(
        title: String,
        width: u32,
        height: u32,
        resizable: bool,
        background: Color,
    ) -> Result<Self, Error> {
        // Same reason as in `Window::create`: keep the X11 compositor in the
        // loop so the window keeps its blur, corners and shadow.
        sdl3::hint::set("SDL_VIDEO_X11_NET_WM_BYPASS_COMPOSITOR", "0");

        let sdl = sdl3::init()?;
        let video = sdl.video()?;

        let mut builder = video.window(&title, width, height);
        if resizable {
            builder.resizable();
        }
        // On macOS the raw handle wgpu sees is the `SDL_MetalView`, and SDL
        // only creates one when asked. The call is a no-op elsewhere.
        builder.metal_view();
        let window = builder.build()?;

        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let surface = create_surface(&instance, &window)?;

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
            apply_limit_buckets: false,
        }))?;

        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("frglib device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            }))?;

        let capabilities = surface.get_capabilities(&adapter);
        // An sRGB target lets wgpu convert the linear colours the shaders
        // produce; without it the fallback format is used as-is.
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .or_else(|| capabilities.formats.first().copied())
            .ok_or(Error::NoSurfaceFormat)?;

        let size = window.size_in_pixels();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width: size.0.max(1),
            height: size.1.max(1),
            // `AutoVsync` falls back to a supported mode instead of failing on
            // a driver that does not offer the preferred one.
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: capabilities
                .alpha_modes
                .first()
                .copied()
                .unwrap_or(wgpu::CompositeAlphaMode::Auto),
            desired_maximum_frame_latency: 2,
            view_formats: Vec::new(),
        };
        surface.configure(&device, &config);

        let adapter_info = adapter.get_info();
        let depth = create_depth_view(&device, (config.width, config.height));
        let gpu = Gpu {
            instance,
            adapter,
            device,
            queue,
            config,
            adapter_info,
        };

        Ok(Self {
            surface,
            window,
            depth,
            gpu,
            background,
            _sdl: sdl,
        })
    }

    /// Drains the event queue and returns everything that happened this frame.
    ///
    /// Works exactly like [`crate::Window::poll`]: SDL's queue is process-wide,
    /// so events are filtered by this window's id.
    pub fn poll(&mut self) -> Result<Vec<Event>, Error> {
        let window_id = self.window.id();

        with_pump(&self._sdl, |pump| {
            let mut events = Vec::new();
            while let Some(event) = pump.poll_event() {
                if let Some(event) = crate::event::translate(event, window_id) {
                    events.push(event);
                }
            }
            Ok(events)
        })
        .map_err(Error::Window)
    }

    /// The GPU handles: instance, adapter, device, queue and surface config.
    pub fn gpu(&self) -> &Gpu {
        &self.gpu
    }

    /// Drawable size in physical pixels.
    pub fn size(&self) -> (u32, u32) {
        self.window.size_in_pixels()
    }

    /// The size `begin_frame` will render at, in physical pixels.
    ///
    /// Follows the last frame, so it is one event behind [`size`](Self::size)
    /// between a resize and the next [`begin_frame`](Self::begin_frame).
    pub fn render_size(&self) -> (u32, u32) {
        self.gpu.size()
    }

    /// Sets the colour every frame is cleared to.
    pub fn set_background(&mut self, color: Color) {
        self.background = color;
    }

    /// The colour every frame is cleared to.
    pub fn background(&self) -> Color {
        self.background
    }

    /// Acquires the next swapchain image and opens a frame for drawing.
    ///
    /// Returns `Ok(None)` when the window is minimised (`Timeout`, `Occluded`)
    /// or the surface needs reconfiguring (`Outdated`, `Lost`); both mean "skip
    /// this frame". Only a validation error is reported: that one is a bug in
    /// the drawing code, not something to retry.
    pub fn begin_frame(&mut self) -> Result<Option<Frame>, Error> {
        let Some(size) = self.sync_surface() else {
            return Ok(None);
        };

        let surface_texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture)
            | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => texture,
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(None)
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.gpu.reconfigure(&self.surface);
                return Ok(None);
            }
            wgpu::CurrentSurfaceTexture::Validation => return Err(Error::SurfaceValidation),
        };

        let color = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frglib frame"),
            });

        Ok(Some(Frame {
            encoder,
            color,
            depth: self.depth.clone(),
            size,
            background: self.background,
            surface_texture,
        }))
    }

    /// Submits the recorded commands and presents the frame.
    ///
    /// The renderer's pass must already be closed; [`Renderer3D::finish`]
    /// (or the caller's own encoder work) is what fills the frame.
    ///
    /// [`Renderer3D::finish`]: crate::gpu::Renderer3D::finish
    pub fn present(&mut self, frame: Frame) -> Result<(), Error> {
        let Frame {
            encoder,
            surface_texture,
            ..
        } = frame;

        self.gpu.queue.submit(std::iter::once(encoder.finish()));
        self.gpu.queue.present(surface_texture);
        Ok(())
    }

    /// Reconfigures the surface when the physical size changed, and returns the
    /// size to render at. `None` means the window has no drawable area.
    fn sync_surface(&mut self) -> Option<(u32, u32)> {
        let size = self.window.size_in_pixels();
        if size.0 == 0 || size.1 == 0 {
            return None;
        }

        if size != self.gpu.size() {
            self.gpu.configure(&self.surface, size);
            self.depth = create_depth_view(self.gpu.device(), size);
        }

        Some(size)
    }
}

/// GPU handles shared by the window and everything drawn through it.
///
/// Cloning the device or the queue is cheap; both are reference-counted
/// handles, which is what lets [`Renderer3D`](super::Renderer3D) keep drawing
/// without borrowing the window.
pub struct Gpu {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    adapter_info: wgpu::AdapterInfo,
}

impl Gpu {
    /// The `wgpu` instance. Mostly useful for creating extra surfaces.
    pub fn instance(&self) -> &wgpu::Instance {
        &self.instance
    }

    /// The chosen adapter, for querying features and limits.
    pub fn adapter(&self) -> &wgpu::Adapter {
        &self.adapter
    }

    /// Name, backend and driver version of the adapter in use.
    pub fn adapter_info(&self) -> &wgpu::AdapterInfo {
        &self.adapter_info
    }

    /// The device custom pipelines are created on.
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// The queue custom passes submit through.
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// The swapchain format. The built-in pipelines are built for this one.
    pub fn format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    /// Drawable size in physical pixels, as of the last configured frame.
    pub fn size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }

    /// The current surface configuration.
    pub fn config(&self) -> &wgpu::SurfaceConfiguration {
        &self.config
    }

    /// Reconfigures the surface for a new physical size.
    ///
    /// Called by the window; exposed crate-wide so the frame code can share it.
    pub(crate) fn configure(&mut self, surface: &wgpu::Surface<'_>, size: (u32, u32)) {
        self.config.width = size.0.max(1);
        self.config.height = size.1.max(1);
        surface.configure(&self.device, &self.config);
    }

    /// Reapplies the current configuration after the surface was lost.
    pub(crate) fn reconfigure(&self, surface: &wgpu::Surface<'_>) {
        surface.configure(&self.device, &self.config);
    }
}

/// One acquired swapchain image and the encoder that draws into it.
///
/// The fields are public on purpose: this is the raw layer. Record the
/// built-in draws with [`Renderer3D`](super::Renderer3D) first -- its pass
/// clears the attachments -- then record custom passes with `LoadOp::Load`,
/// and finally hand the frame back to
/// [`WgpuWindow::present`](WgpuWindow::present).
pub struct Frame {
    /// The command encoder for this frame.
    pub encoder: wgpu::CommandEncoder,
    /// The swapchain image, ready as a render target.
    pub color: wgpu::TextureView,
    /// The depth buffer matching [`color`](Self::color).
    pub depth: wgpu::TextureView,
    /// Size of both attachments, in physical pixels.
    pub size: (u32, u32),
    /// Colour the built-in renderer clears to.
    pub background: Color,
    /// Presented by [`WgpuWindow::present`](WgpuWindow::present).
    pub(crate) surface_texture: wgpu::SurfaceTexture,
}

impl Frame {
    /// Width divided by height, for the camera projection.
    pub fn aspect(&self) -> f32 {
        self.size.0 as f32 / self.size.1.max(1) as f32
    }
}

fn create_depth_view(device: &wgpu::Device, size: (u32, u32)) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("frglib depth"),
        size: wgpu::Extent3d {
            width: size.0.max(1),
            height: size.1.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });

    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

/// Creates the surface and stretches its lifetime to `'static`.
///
/// SDL's `Window` is `!Send`/`!Sync`, which is why the handle is wrapped; the
/// unchecked `Send`/`Sync` is sound because the window stays on the same
/// thread and is not touched from another one. The lifetime is then stretched
/// because `WgpuWindow` stores the surface and the window together, with the
/// surface declared first so it is dropped first.
fn create_surface(
    instance: &wgpu::Instance,
    window: &SdlWindow,
) -> Result<wgpu::Surface<'static>, Error> {
    struct SyncWindow<'a>(&'a SdlWindow);

    // SAFETY: see the function documentation.
    unsafe impl Send for SyncWindow<'_> {}
    // SAFETY: see the function documentation.
    unsafe impl Sync for SyncWindow<'_> {}

    impl wgpu::rwh::HasWindowHandle for SyncWindow<'_> {
        fn window_handle(&self) -> Result<wgpu::rwh::WindowHandle<'_>, wgpu::rwh::HandleError> {
            self.0.window_handle()
        }
    }

    impl wgpu::rwh::HasDisplayHandle for SyncWindow<'_> {
        fn display_handle(&self) -> Result<wgpu::rwh::DisplayHandle<'_>, wgpu::rwh::HandleError> {
            self.0.display_handle()
        }
    }

    let surface = instance.create_surface(SyncWindow(window))?;

    // SAFETY: `WgpuWindow` owns `window` for at least as long as `surface`, and
    // declares `surface` before `window`, so the surface goes first.
    Ok(unsafe { std::mem::transmute::<wgpu::Surface<'_>, wgpu::Surface<'static>>(surface) })
}
