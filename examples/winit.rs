#![cfg(windows)]

use std::{ptr, time::Instant};

use imgui::{FontConfig, FontSource};
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Direct3D9::{
    Direct3DCreate9, IDirect3D9, IDirect3DDevice9, D3D_SDK_VERSION, D3DADAPTER_DEFAULT,
    D3DCLEAR_TARGET, D3DCREATE_SOFTWARE_VERTEXPROCESSING, D3DDEVTYPE_HAL, D3DFMT_R5G6B5,
    D3DMULTISAMPLE_NONE, D3DPRESENT_INTERVAL_DEFAULT, D3DPRESENT_PARAMETERS,
    D3DPRESENT_RATE_DEFAULT, D3DSWAPEFFECT_DISCARD,
};
use windows_core::BOOL;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};
use winit::event::Event;

const WINDOW_WIDTH: f64 = 760.0;
const WINDOW_HEIGHT: f64 = 760.0;

unsafe fn set_up_dx_context(hwnd: HWND) -> (IDirect3D9, IDirect3DDevice9) {
    let d9_option = unsafe { Direct3DCreate9(D3D_SDK_VERSION) };
    match d9_option {
        Some(d9) => {
            let mut present_params = D3DPRESENT_PARAMETERS {
                BackBufferCount: 1,
                MultiSampleType: D3DMULTISAMPLE_NONE,
                MultiSampleQuality: 0,
                SwapEffect: D3DSWAPEFFECT_DISCARD,
                hDeviceWindow: hwnd,
                Flags: 0,
                FullScreen_RefreshRateInHz: D3DPRESENT_RATE_DEFAULT,
                PresentationInterval: D3DPRESENT_INTERVAL_DEFAULT as u32,
                BackBufferFormat: D3DFMT_R5G6B5,
                EnableAutoDepthStencil: BOOL(0),
                Windowed: BOOL(1),
                BackBufferWidth: WINDOW_WIDTH as _,
                BackBufferHeight: WINDOW_HEIGHT as _,
                ..unsafe { core::mem::zeroed() }
            };
            let mut device: Option<IDirect3DDevice9> = None;
            unsafe {
                match d9.CreateDevice(
                    D3DADAPTER_DEFAULT,
                    D3DDEVTYPE_HAL,
                    hwnd,
                    D3DCREATE_SOFTWARE_VERTEXPROCESSING as u32,
                    &mut present_params,
                    &mut device,
                ) {
                    Ok(_) => (d9, device.unwrap()),
                    _ => panic!("CreateDevice failed"),
                }
            }
        },
        None => panic!("Direct3DCreate9 failed"),
    }
}

struct App {
    window: Option<Window>,
    imgui: imgui::Context,
    platform: Option<WinitPlatform>,
    renderer: Option<imgui_dx9_renderer::Renderer>,
    d9: Option<IDirect3D9>,
    device: Option<IDirect3DDevice9>,
    last_frame: Instant,
}

impl App {
    fn new() -> Self {
        let mut imgui = imgui::Context::create();
        imgui.set_ini_filename(None);

        Self {
            window: None,
            imgui,
            platform: None,
            renderer: None,
            d9: None,
            device: None,
            last_frame: Instant::now(),
        }
    }
}

impl ApplicationHandler for App {
    fn new_events(&mut self, _event_loop: &ActiveEventLoop, _cause: winit::event::StartCause) {
        let now = Instant::now();
        self.imgui.io_mut().update_delta_time(now - self.last_frame);
        self.last_frame = now;
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window = event_loop
            .create_window(
                Window::default_attributes()
                    .with_title("imgui_dx9_renderer winit example")
                    .with_resizable(false)
                    .with_inner_size(LogicalSize {
                        width: WINDOW_WIDTH,
                        height: WINDOW_HEIGHT,
                    }),
            )
            .unwrap();

        let hwnd = match window.window_handle().unwrap().as_raw() {
            RawWindowHandle::Win32(handle) => HWND(handle.hwnd.get() as _),
            _ => unreachable!(),
        };

        let (d9, device) = unsafe { set_up_dx_context(hwnd) };

        let mut platform = WinitPlatform::new(&mut self.imgui);
        platform.attach_window(self.imgui.io_mut(), &window, HiDpiMode::Rounded);

        let hidpi_factor = platform.hidpi_factor();
        let font_size = (13.0 * hidpi_factor) as f32;
        self.imgui.fonts().add_font(&[FontSource::DefaultFontData {
            config: Some(FontConfig {
                size_pixels: font_size,
                ..FontConfig::default()
            }),
        }]);
        self.imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;

        let renderer =
            unsafe { imgui_dx9_renderer::Renderer::new(&mut self.imgui, device.clone()).unwrap() };

        self.window = Some(window);
        self.platform = Some(platform);
        self.renderer = Some(renderer);
        self.d9 = Some(d9);
        self.device = Some(device);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        let Some(window) = self.window.as_ref() else {
            return;
        };

        let event: Event<WindowEvent> = Event::WindowEvent { window_id, event };

        if let Some(platform) = self.platform.as_mut() {
            platform.handle_event(self.imgui.io_mut(), window, &event);
        }

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => event_loop.exit(),
            Event::WindowEvent {
                event: WindowEvent::RedrawRequested,
                ..
            } => {
                let Some(device) = self.device.as_ref() else {
                    return;
                };
                let Some(platform) = self.platform.as_mut() else {
                    return;
                };
                let Some(renderer) = self.renderer.as_mut() else {
                    return;
                };

                unsafe {
                    device
                        .Clear(0, ptr::null_mut(), D3DCLEAR_TARGET as u32, 0xFFAA_AAAA, 1.0, 0)
                        .unwrap();
                    device.BeginScene().unwrap();
                }

                let ui = self.imgui.new_frame();
                ui.window("Hello world")
                    .size([300.0, 100.0], imgui::Condition::FirstUseEver)
                    .build(|| {
                        ui.text("Hello world!");
                        ui.text("This...is...imgui-rs!");
                        ui.separator();
                        let mouse_pos = ui.io().mouse_pos;
                        ui.text(format!(
                            "Mouse Position: ({:.1},{:.1})",
                            mouse_pos[0], mouse_pos[1]
                        ));
                    });
                ui.show_demo_window(&mut true);

                platform.prepare_render(ui, window);
                renderer.render(self.imgui.render()).unwrap();

                unsafe {
                    device.EndScene().unwrap();
                    device
                        .Present(ptr::null_mut(), ptr::null_mut(), HWND::default(), ptr::null_mut())
                        .unwrap();
                }
            },
            _ => {},
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let Some(platform) = self.platform.as_mut() else {
            return;
        };

        platform
            .prepare_frame(self.imgui.io_mut(), window)
            .expect("Failed to start frame");
        window.request_redraw();
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new();
    event_loop.run_app(&mut app).unwrap();
}
