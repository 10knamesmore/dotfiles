pub mod displays;
pub mod events;
mod ffi;
mod windows;

use std::ptr::{NonNull, null};

use anyhow::{Context, Result, ensure};
use objc2_core_foundation::{
    CFArray, CFDictionary, CFNumber, CFRetained, CFString, CGPoint, CGRect,
};
use objc2_core_graphics::{CGContext, CGMutablePath};

use crate::{
    color::Color,
    geometry::{BorderGeometry, rect},
    paint::BorderPaint,
};
pub use ffi::WindowId;
use ffi::*;

fn check(code: i32, operation: &str) -> Result<()> {
    ensure!(code == 0, "{operation}: CGError {code}");
    Ok(())
}

/// Adopts a +1 CoreFoundation result from a private Create/Copy API.
unsafe fn owned<T: objc2_core_foundation::Type>(
    pointer: *mut T,
    operation: &str,
) -> Result<CFRetained<T>> {
    let pointer = NonNull::new(pointer).with_context(|| format!("{operation} returned null"))?;
    Ok(unsafe { CFRetained::from_raw(pointer) })
}

#[derive(Clone, Copy)]
pub struct Connection {
    id: ConnectionId,
}

#[derive(Clone, Debug)]
pub struct TargetWindow {
    pub frame: CGRect,
    pub level: i32,
    pub shown: bool,
    pub spaces: Vec<u64>,
    pub sublevel: i32,
    pub sticky: bool,
    pub fullscreen: bool,
    pub eligible: bool,
}

#[derive(Clone, Copy)]
pub struct Placement {
    pub target: WindowId,
    pub space: u64,
    pub level: i32,
    pub sublevel: i32,
    pub sticky: bool,
}

impl Connection {
    pub fn main() -> Self {
        Self {
            id: unsafe { SLSMainConnectionID() },
        }
    }

    pub fn place(
        &self,
        surfaces: &[Surface],
        retiring: &[Surface],
        geometry: &BorderGeometry,
        placement: &Placement,
    ) -> Result<()> {
        if surfaces.is_empty() {
            return Ok(());
        }
        unsafe {
            let transaction = owned(
                SLSTransactionCreate(self.id),
                "create placement transaction",
            )?;
            for surface in retiring {
                SLSTransactionOrderWindow(&transaction, surface.id, 0, placement.target);
            }
            for (surface, viewport) in surfaces.iter().zip(&geometry.surfaces) {
                let origin = CGPoint::new(
                    geometry.screen_frame.origin.x + viewport.origin.x,
                    geometry.screen_frame.origin.y + viewport.origin.y,
                );
                SLSTransactionMoveWindowWithGroup(&transaction, surface.id, origin);
                SLSTransactionSetWindowLevel(&transaction, surface.id, placement.level);
                SLSTransactionSetWindowSubLevel(&transaction, surface.id, placement.sublevel);
                SLSTransactionOrderWindow(&transaction, surface.id, -1, placement.target);
            }
            SLSTransactionCommit(&transaction, 0);
        }
        Ok(())
    }

    pub fn set_membership(&self, surfaces: &[Surface], placement: &Placement) -> Result<()> {
        if surfaces.is_empty() {
            return Ok(());
        }
        let windows = window_array(
            &surfaces
                .iter()
                .map(|surface| surface.id)
                .collect::<Vec<_>>(),
        );
        unsafe {
            SLSMoveWindowsToManagedSpace(self.id, windows.as_opaque(), placement.space);
            for surface in surfaces {
                let sticky = 1 << 11;
                if placement.sticky {
                    check(
                        SLSSetWindowTags(self.id, surface.id, &sticky, 64),
                        "set sticky border",
                    )?;
                    check(
                        SLSClearWindowTags(self.id, surface.id, &(1 << 45), 64),
                        "allow border on all Spaces",
                    )?;
                } else {
                    check(
                        SLSClearWindowTags(self.id, surface.id, &sticky, 64),
                        "clear sticky border",
                    )?;
                }
            }
        }
        Ok(())
    }
}

fn window_array(windows: &[WindowId]) -> CFRetained<CFArray<CFNumber>> {
    CFArray::from_retained_objects(
        &windows
            .iter()
            .map(|&id| CFNumber::new_i32(id as i32))
            .collect::<Vec<_>>(),
    )
}

/// Owns one fixed-size window. Resizing replaces it; moving reuses its pixels.
pub struct Surface {
    connection: Connection,
    pub id: WindowId,
}

impl Surface {
    pub fn draw(
        connection: Connection,
        geometry: &BorderGeometry,
        viewport: CGRect,
        paint: BorderPaint,
    ) -> Result<Self> {
        let surface = Self::new(connection, viewport, geometry.scale)?;
        surface.repaint(geometry, viewport, paint)?;
        Ok(surface)
    }

    fn new(connection: Connection, viewport: CGRect, scale: f64) -> Result<Self> {
        let bounds = rect(0.0, 0.0, viewport.size.width, viewport.size.height);
        let mut region = std::ptr::null_mut();
        let mut id = 0;
        unsafe {
            check(
                CGSNewRegionWithRect(&bounds, &mut region),
                "create surface region",
            )?;
            let region = owned(region, "create surface region")?;
            check(
                SLSNewWindow(connection.id, 2, -9999.0, -9999.0, &region, &mut id),
                "create buffered window",
            )?;
        }
        let surface = Self { connection, id };
        unsafe {
            // SkyLight tags: floating (bit 1), ignores mouse events (bit 9).
            check(
                SLSSetWindowTags(connection.id, id, &((1 << 1) | (1 << 9)), 64),
                "set border window tags",
            )?;
            check(
                SLSSetWindowResolution(connection.id, id, scale),
                "set surface resolution",
            )?;
            check(
                SLSSetWindowOpacity(connection.id, id, false),
                "set surface transparency",
            )?;
            let shadow_key = CFString::from_str("com.apple.WindowShadowDensity");
            let zero = CFNumber::new_i32(0);
            let properties = CFDictionary::from_slices(&[&*shadow_key], &[&*zero]);
            check(
                SLSWindowSetShadowProperties(id, properties.as_opaque()),
                "disable window shadow",
            )?;
        }
        Ok(surface)
    }

    pub fn repaint(
        &self,
        geometry: &BorderGeometry,
        viewport: CGRect,
        paint: BorderPaint,
    ) -> Result<()> {
        unsafe {
            let context = owned(
                SLWindowContextCreate(self.connection.id, self.id, null()),
                "create window CGContext",
            )?;
            draw_ring(&context, geometry, viewport, paint);
            check(
                SLSFlushWindowContentRegion(self.connection.id, self.id, null()),
                "flush border pixels",
            )?;
        }
        Ok(())
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            SLSReleaseWindow(self.connection.id, self.id);
        }
    }
}

fn draw_ring(context: &CGContext, geometry: &BorderGeometry, viewport: CGRect, paint: BorderPaint) {
    let context = Some(context);
    CGContext::clear_rect(
        context,
        rect(0.0, 0.0, viewport.size.width, viewport.size.height),
    );
    CGContext::save_g_state(context);
    CGContext::translate_ctm(context, 0.0, viewport.size.height);
    CGContext::scale_ctm(context, 1.0, -1.0);
    CGContext::translate_ctm(context, -viewport.origin.x, -viewport.origin.y);
    let path = CGMutablePath::new();
    // Every surface draws the same path through a disjoint viewport.
    unsafe {
        CGMutablePath::add_rounded_rect(
            Some(&path),
            null(),
            geometry.outer,
            geometry.radius + geometry.width,
            geometry.radius + geometry.width,
        );
        CGMutablePath::add_rounded_rect(
            Some(&path),
            null(),
            geometry.inner,
            geometry.radius,
            geometry.radius,
        );
    }
    CGContext::add_path(context, Some(&path));
    CGContext::eo_clip(context);
    fill_rect(context, viewport, paint.color);
    if let Some(segments) = paint.top_segments {
        let top = geometry.straight_top_edge();
        CGContext::clear_rect(context, top);
        let slot_width = top.size.width / segments.count as f64;
        let gap = 6.0_f64.min(slot_width / 3.0);
        for index in 0..segments.count {
            let left_gap = if index == 0 { 0.0 } else { gap / 2.0 };
            let right_gap = if index + 1 == segments.count {
                0.0
            } else {
                gap / 2.0
            };
            let segment = rect(
                top.origin.x + slot_width * index as f64 + left_gap,
                top.origin.y,
                slot_width - left_gap - right_gap,
                top.size.height,
            );
            let color = if index == segments.active_index {
                paint.color
            } else {
                segments.inactive_color
            };
            fill_rect(context, segment, color);
        }
    }
    CGContext::restore_g_state(context);
    CGContext::flush(context);
}

fn fill_rect(context: Option<&CGContext>, bounds: CGRect, color: Color) {
    let [red, green, blue, alpha] = color.rgba();
    CGContext::set_rgb_fill_color(context, red, green, blue, alpha);
    CGContext::fill_rect(context, bounds);
}
