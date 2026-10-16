//! Shared Direct2D drawing of the reference scene (spikes B and C).

use windows::Win32::Graphics::Direct2D::Common::{
    D2D_RECT_F, D2D1_BEZIER_SEGMENT, D2D1_COLOR_F, D2D1_FIGURE_BEGIN_FILLED,
    D2D1_FIGURE_BEGIN_HOLLOW, D2D1_FIGURE_END_CLOSED, D2D1_FIGURE_END_OPEN,
};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D_SIZE_U, D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_PIXEL_FORMAT,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1_BITMAP_INTERPOLATION_MODE_LINEAR, D2D1_BITMAP_PROPERTIES,
    D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT, D2D1_ELLIPSE, D2D1_ROUNDED_RECT, ID2D1Bitmap,
    ID2D1Factory, ID2D1PathGeometry, ID2D1RenderTarget, ID2D1SolidColorBrush,
};
use windows::Win32::Graphics::DirectWrite::DWRITE_TEXT_RANGE;
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows_numerics::{Matrix3x2, Vector2};

use crate::scene::{self, Scene};

pub fn color(c: u32, a: f32) -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        r: ((c >> 16) & 0xFF) as f32 / 255.0,
        g: ((c >> 8) & 0xFF) as f32 / 255.0,
        b: (c & 0xFF) as f32 / 255.0,
        a,
    }
}

fn pt(x: f32, y: f32) -> Vector2 {
    Vector2 { X: x, Y: y }
}

/// Device-dependent resources for one render target.
pub struct Res {
    brushes: Vec<(u32, ID2D1SolidColorBrush)>,
    sel: ID2D1SolidColorBrush,
    triangle: ID2D1PathGeometry,
    bezier: ID2D1PathGeometry,
    bitmap: ID2D1Bitmap,
}

impl Res {
    fn brush(&self, c: u32) -> Option<&ID2D1SolidColorBrush> {
        self.brushes.iter().find(|(k, _)| *k == c).map(|(_, b)| b)
    }
}

/// Creates brushes, geometry and the image bitmap; sets per-range drawing effects on the layouts.
pub fn create(f: &ID2D1Factory, rt: &ID2D1RenderTarget, s: &Scene) -> windows::core::Result<Res> {
    unsafe {
        let mut colors = vec![
            scene::TEXT,
            scene::LINK,
            scene::CODE_BG,
            scene::CODE_BORDER,
            scene::RULE,
            scene::BG,
        ];
        for t in &s.texts {
            colors.push(t.color);
            colors.extend(t.ranges.iter().map(|r| r.color));
        }
        colors.sort_unstable();
        colors.dedup();
        let mut brushes = Vec::new();
        for c in colors {
            brushes.push((c, rt.CreateSolidColorBrush(&color(c, 1.0), None)?));
        }
        let res_brush = |c: u32| {
            brushes
                .iter()
                .find(|(k, _)| *k == c)
                .map(|(_, b)| b.clone())
        };
        for t in &s.texts {
            for r in &t.ranges {
                if let Some(b) = res_brush(r.color) {
                    t.layout.SetDrawingEffect(
                        &b,
                        DWRITE_TEXT_RANGE {
                            startPosition: r.start,
                            length: r.len,
                        },
                    )?;
                }
            }
        }
        let sel = rt.CreateSolidColorBrush(&color(scene::SEL, scene::SEL_ALPHA), None)?;
        let (x0, py) = (scene::X0, s.prim_y);

        let triangle = f.CreatePathGeometry()?;
        let sink = triangle.Open()?;
        sink.BeginFigure(pt(x0 + 200.0, py + 140.0), D2D1_FIGURE_BEGIN_FILLED);
        sink.AddLine(pt(x0 + 186.0, py + 137.0));
        sink.AddLine(pt(x0 + 196.0, py + 127.0));
        sink.EndFigure(D2D1_FIGURE_END_CLOSED);
        sink.Close()?;

        let bezier = f.CreatePathGeometry()?;
        let sink = bezier.Open()?;
        sink.BeginFigure(pt(x0 + 240.0, py + 130.0), D2D1_FIGURE_BEGIN_HOLLOW);
        sink.AddBezier(&D2D1_BEZIER_SEGMENT {
            point1: pt(x0 + 280.0, py + 10.0),
            point2: pt(x0 + 380.0, py + 150.0),
            point3: pt(x0 + 460.0, py + 30.0),
        });
        sink.EndFigure(D2D1_FIGURE_END_OPEN);
        sink.Close()?;

        let dpi = 96.0 * s.scale;
        let bitmap = rt.CreateBitmap(
            D2D_SIZE_U {
                width: s.img_w,
                height: s.img_h,
            },
            Some(s.img.as_ptr().cast()),
            s.img_w * 4,
            &D2D1_BITMAP_PROPERTIES {
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                dpiX: dpi,
                dpiY: dpi,
            },
        )?;
        Ok(Res {
            brushes,
            sel,
            triangle,
            bezier,
            bitmap,
        })
    }
}

/// Draws the rows `[top, top + rows)` (global device px) of the scene into `rt`, whose DPI is
/// already `96 * scale`. Caller wraps this in `BeginDraw`/`EndDraw`.
pub fn draw(rt: &ID2D1RenderTarget, res: &Res, s: &Scene, top: f32, rows: f32) {
    unsafe {
        rt.Clear(Some(&color(scene::BG, 1.0)));
        for i in s.visible(top, top + rows) {
            let off = (i as f32 * s.inst_px() - top) / s.scale;
            rt.SetTransform(&Matrix3x2::translation(0.0, off));
            draw_instance(rt, res, s);
        }
        rt.SetTransform(&Matrix3x2::identity());
    }
}

unsafe fn draw_instance(rt: &ID2D1RenderTarget, res: &Res, s: &Scene) {
    let (Some(text), Some(rule), Some(code_bg), Some(code_border), Some(link)) = (
        res.brush(scene::TEXT),
        res.brush(scene::RULE),
        res.brush(scene::CODE_BG),
        res.brush(scene::CODE_BORDER),
        res.brush(scene::LINK),
    ) else {
        return;
    };
    unsafe {
        for r in &s.selection {
            rt.FillRectangle(
                &D2D_RECT_F {
                    left: r.x,
                    top: r.y,
                    right: r.x + r.w,
                    bottom: r.y + r.h,
                },
                &res.sel,
            );
        }
        let c = s.code_box;
        let rr = D2D1_ROUNDED_RECT {
            rect: D2D_RECT_F {
                left: c.x + 0.5,
                top: c.y + 0.5,
                right: c.x + c.w - 0.5,
                bottom: c.y + c.h - 0.5,
            },
            radiusX: 8.0,
            radiusY: 8.0,
        };
        rt.FillRoundedRectangle(&rr, code_bg);
        rt.DrawRoundedRectangle(&rr, code_border, 1.0, None);

        let t = s.table;
        rt.FillRectangle(
            &D2D_RECT_F {
                left: t.x,
                top: t.y,
                right: t.x + t.w,
                bottom: t.y + scene::TABLE_ROW_H,
            },
            code_bg,
        );
        for row in 0..=3 {
            let y = t.y + row as f32 * scene::TABLE_ROW_H + 0.5;
            rt.DrawLine(pt(t.x, y), pt(t.x + t.w, y), rule, 1.0, None);
        }
        for col in 0..=4 {
            let x = t.x + col as f32 * scene::TABLE_COL_W + 0.5;
            rt.DrawLine(pt(x, t.y), pt(x, t.y + t.h), rule, 1.0, None);
        }

        let (x0, py) = (scene::X0, s.prim_y);
        rt.FillRectangle(
            &D2D_RECT_F {
                left: x0,
                top: py,
                right: x0 + 3.0,
                bottom: py + scene::PRIM_H,
            },
            rule,
        );
        rt.FillEllipse(
            &D2D1_ELLIPSE {
                point: pt(x0 + 40.0, py + 20.0),
                radiusX: 6.0,
                radiusY: 6.0,
            },
            link,
        );
        rt.DrawLine(
            pt(x0 + 20.0, py + 40.0),
            pt(x0 + 200.0, py + 140.0),
            text,
            1.5,
            None,
        );
        rt.FillGeometry(&res.triangle, text, None);
        rt.DrawGeometry(&res.bezier, link, 1.5, None);
        rt.DrawBitmap(
            &res.bitmap,
            Some(&D2D_RECT_F {
                left: x0 + scene::IMG_X,
                top: py,
                right: x0 + scene::IMG_X + scene::IMG_W,
                bottom: py + scene::IMG_H,
            }),
            1.0,
            D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
            None,
        );

        for tx in &s.texts {
            let b = res.brush(tx.color).unwrap_or(text);
            rt.DrawTextLayout(
                Vector2 { X: tx.x, Y: tx.y },
                &tx.layout,
                b,
                D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT,
            );
        }
    }
}
