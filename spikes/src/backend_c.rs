//! Spike C: Direct2D device context on a D3D11 device (hardware, WARP fallback) presenting via a
//! flip-model swap chain; scrolling redraws the full frame (§4.7 candidate C).

use windows::Win32::Foundation::{HMODULE, HWND, RECT};
use windows::Win32::Graphics::Direct2D::Common::{D2D1_ALPHA_MODE_IGNORE, D2D1_PIXEL_FORMAT};
use windows::Win32::Graphics::Direct2D::{
    D2D1_BITMAP_OPTIONS_CANNOT_DRAW, D2D1_BITMAP_OPTIONS_TARGET, D2D1_BITMAP_PROPERTIES1,
    D2D1_DEVICE_CONTEXT_OPTIONS_NONE, D2D1_FACTORY_TYPE_SINGLE_THREADED,
    D2D1_TEXT_ANTIALIAS_MODE_CLEARTYPE, D2D1CreateFactory, ID2D1DeviceContext, ID2D1Factory,
    ID2D1Factory1, ID2D1RenderTarget,
};
use windows::Win32::Graphics::Direct3D::{
    D3D_DRIVER_TYPE, D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP,
};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION, D3D11CreateDevice, ID3D11Device,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_ALPHA_MODE_IGNORE, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_UNKNOWN, DXGI_SAMPLE_DESC,
};
use windows::Win32::Graphics::Dxgi::{
    DXGI_MWA_NO_ALT_ENTER, DXGI_PRESENT, DXGI_SCALING_NONE, DXGI_SWAP_CHAIN_DESC1,
    DXGI_SWAP_CHAIN_FLAG, DXGI_SWAP_EFFECT_FLIP_DISCARD, DXGI_USAGE_RENDER_TARGET_OUTPUT,
    IDXGIDevice, IDXGIFactory2, IDXGISurface, IDXGISwapChain1,
};
use windows::Win32::Graphics::Gdi::HDC;
use windows::Win32::UI::WindowsAndMessaging::{
    GetClientRect, WINDOW_EX_STYLE, WS_EX_NOREDIRECTIONBITMAP,
};
use windows::core::Interface;

use crate::app::{Log, ScrollMode};
use crate::scene::Scene;

pub struct BackendC {
    hwnd: HWND,
    dc: ID2D1DeviceContext,
    sc: IDXGISwapChain1,
    res: crate::d2d::Res,
    dpi: f32,
    h: u32,
}

fn device(kind: D3D_DRIVER_TYPE) -> windows::core::Result<ID3D11Device> {
    let mut dev = None;
    unsafe {
        D3D11CreateDevice(
            None,
            kind,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            None,
            D3D11_SDK_VERSION,
            Some(&mut dev),
            None,
            None,
        )?;
    }
    dev.ok_or_else(windows::core::Error::empty)
}

impl BackendC {
    fn bind_target(&self) -> windows::core::Result<()> {
        unsafe {
            let surface: IDXGISurface = self.sc.GetBuffer(0)?;
            let bmp = self.dc.CreateBitmapFromDxgiSurface(
                &surface,
                Some(&D2D1_BITMAP_PROPERTIES1 {
                    pixelFormat: D2D1_PIXEL_FORMAT {
                        format: DXGI_FORMAT_B8G8R8A8_UNORM,
                        alphaMode: D2D1_ALPHA_MODE_IGNORE,
                    },
                    dpiX: self.dpi,
                    dpiY: self.dpi,
                    bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
                    colorContext: std::mem::ManuallyDrop::new(None),
                }),
            )?;
            self.dc.SetTarget(&bmp);
            self.dc.SetDpi(self.dpi, self.dpi);
        }
        Ok(())
    }

    fn frame(&mut self, s: &Scene, scroll: i32) {
        let Ok(base) = self.dc.cast::<ID2D1RenderTarget>() else {
            return;
        };
        unsafe {
            base.BeginDraw();
            crate::d2d::draw(&base, &self.res, s, scroll as f32, self.h as f32);
            let _ = base.EndDraw(None, None);
            let _ = self.sc.Present(0, DXGI_PRESENT(0));
        }
    }
}

impl crate::app::Backend for BackendC {
    const NAME: &'static str = "C (D2D DeviceContext, flip swap chain)";
    const MODE: ScrollMode = ScrollMode::Redraw;
    const EX_STYLE: WINDOW_EX_STYLE = WS_EX_NOREDIRECTIONBITMAP;

    fn create(hwnd: HWND, s: &Scene, w: u32, h: u32, log: &mut Log) -> windows::core::Result<Self> {
        unsafe {
            let (dev, kind) = match device(D3D_DRIVER_TYPE_HARDWARE) {
                Ok(d) => (d, "hardware"),
                Err(_) => (device(D3D_DRIVER_TYPE_WARP)?, "warp"),
            };
            log.kv("d3d.driver", kind);
            log.pws("d3d_device");
            let dxgi: IDXGIDevice = dev.cast()?;
            let adapter = dxgi.GetAdapter()?;
            let factory: IDXGIFactory2 = adapter.GetParent()?;
            let sc = factory.CreateSwapChainForHwnd(
                &dev,
                hwnd,
                &DXGI_SWAP_CHAIN_DESC1 {
                    Width: w,
                    Height: h,
                    Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    SampleDesc: DXGI_SAMPLE_DESC {
                        Count: 1,
                        Quality: 0,
                    },
                    BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
                    BufferCount: 2,
                    Scaling: DXGI_SCALING_NONE,
                    SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
                    AlphaMode: DXGI_ALPHA_MODE_IGNORE,
                    ..Default::default()
                },
                None,
                None,
            )?;
            let _ = factory.MakeWindowAssociation(hwnd, DXGI_MWA_NO_ALT_ENTER);
            log.pws("swap_chain");
            let f1: ID2D1Factory1 = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            let d2dev = f1.CreateDevice(&dxgi)?;
            let dc = d2dev.CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)?;
            dc.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_CLEARTYPE);
            log.pws("d2d_device_context");
            let base: ID2D1RenderTarget = dc.cast()?;
            let f: ID2D1Factory = f1.cast()?;
            let res = crate::d2d::create(&f, &base, s)?;
            let me = Self {
                hwnd,
                dc,
                sc,
                res,
                dpi: 96.0 * s.scale,
                h,
            };
            me.bind_target()?;
            Ok(me)
        }
    }

    fn resize(&mut self, w: u32, h: u32) {
        if w == 0 || h == 0 {
            return;
        }
        self.h = h;
        unsafe {
            self.dc.SetTarget(None);
            let _ = self
                .sc
                .ResizeBuffers(0, w, h, DXGI_FORMAT_UNKNOWN, DXGI_SWAP_CHAIN_FLAG(0));
        }
        let _ = self.bind_target();
    }

    fn paint(&mut self, s: &Scene, scroll: i32, _rc: RECT, _hdc: HDC) {
        let mut rc = RECT::default();
        unsafe {
            let _ = GetClientRect(self.hwnd, &mut rc);
        }
        self.h = rc.bottom.max(1) as u32;
        self.frame(s, scroll);
    }

    fn present_full(&mut self, s: &Scene, scroll: i32) {
        self.frame(s, scroll);
    }
}
