//! Spike B: Direct2D DC render target in **software** mode, rendered into a banded DIB and
//! `BitBlt`ed to the window; scrolling via `ScrollWindowEx` (§4.7 candidate B).

use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Direct2D::Common::{D2D1_ALPHA_MODE_IGNORE, D2D1_PIXEL_FORMAT};
use windows::Win32::Graphics::Direct2D::{
    D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_FEATURE_LEVEL_DEFAULT, D2D1_RENDER_TARGET_PROPERTIES,
    D2D1_RENDER_TARGET_TYPE_SOFTWARE, D2D1_RENDER_TARGET_USAGE_NONE,
    D2D1_TEXT_ANTIALIAS_MODE_CLEARTYPE, D2D1CreateFactory, ID2D1DCRenderTarget, ID2D1Factory,
    ID2D1RenderTarget,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CreateCompatibleDC, CreateDIBSection,
    DIB_RGB_COLORS, DeleteDC, DeleteObject, HBITMAP, HDC, HGDIOBJ, SRCCOPY, SelectObject,
};
use windows::Win32::UI::WindowsAndMessaging::WINDOW_EX_STYLE;
use windows::core::Interface;

use crate::app::{Log, ScrollMode};
use crate::scene::Scene;

pub const BAND: i32 = 128;

pub struct Band {
    pub dc: HDC,
    pub bmp: HBITMAP,
    pub old: HGDIOBJ,
    pub bits: *mut u8,
    pub w: i32,
}

impl Band {
    pub fn new(w: i32) -> Self {
        unsafe {
            let dc = CreateCompatibleDC(None);
            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: w.max(1),
                    biHeight: -BAND,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bits = std::ptr::null_mut();
            let bmp = CreateDIBSection(Some(dc), &bmi, DIB_RGB_COLORS, &mut bits, None, 0)
                .unwrap_or_default();
            let old = SelectObject(dc, bmp.into());
            Self {
                dc,
                bmp,
                old,
                bits: bits.cast(),
                w: w.max(1),
            }
        }
    }
}

impl Drop for Band {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.old);
            let _ = DeleteObject(self.bmp.into());
            let _ = DeleteDC(self.dc);
        }
    }
}

pub struct BackendB {
    rt: ID2D1DCRenderTarget,
    res: crate::d2d::Res,
    band: Band,
}

impl crate::app::Backend for BackendB {
    const NAME: &'static str = "B (D2D DC RT, software)";
    const MODE: ScrollMode = ScrollMode::Blit;
    const EX_STYLE: WINDOW_EX_STYLE = WINDOW_EX_STYLE(0);

    fn create(
        _hwnd: HWND,
        s: &Scene,
        w: u32,
        _h: u32,
        log: &mut Log,
    ) -> windows::core::Result<Self> {
        unsafe {
            let f: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            log.pws("d2d_factory");
            let dpi = 96.0 * s.scale;
            let rt = f.CreateDCRenderTarget(&D2D1_RENDER_TARGET_PROPERTIES {
                r#type: D2D1_RENDER_TARGET_TYPE_SOFTWARE,
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_IGNORE,
                },
                dpiX: dpi,
                dpiY: dpi,
                usage: D2D1_RENDER_TARGET_USAGE_NONE,
                minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
            })?;
            rt.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_CLEARTYPE);
            let base: ID2D1RenderTarget = rt.cast()?;
            let res = crate::d2d::create(&f, &base, s)?;
            Ok(Self {
                rt,
                res,
                band: Band::new(w as i32),
            })
        }
    }

    fn resize(&mut self, w: u32, _h: u32) {
        if w as i32 != self.band.w {
            self.band = Band::new(w as i32);
        }
    }

    fn paint(&mut self, s: &Scene, scroll: i32, rc: RECT, hdc: HDC) {
        let Ok(base) = self.rt.cast::<ID2D1RenderTarget>() else {
            return;
        };
        let w = self.band.w;
        let mut y = rc.top;
        while y < rc.bottom {
            let bh = BAND.min(rc.bottom - y);
            unsafe {
                let r = RECT {
                    left: 0,
                    top: 0,
                    right: w,
                    bottom: bh,
                };
                if self.rt.BindDC(self.band.dc, &r).is_err() {
                    return;
                }
                base.BeginDraw();
                crate::d2d::draw(&base, &self.res, s, (scroll + y) as f32, bh as f32);
                let _ = base.EndDraw(None, None);
                let _ = BitBlt(hdc, 0, y, w, bh, Some(self.band.dc), 0, 0, SRCCOPY);
            }
            y += bh;
        }
    }
}
