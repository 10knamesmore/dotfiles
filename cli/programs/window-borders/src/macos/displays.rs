use anyhow::{Context, Result};
use objc2_core_foundation::{CFUUID, CGRect};
use objc2_core_graphics::{
    CGDisplayBounds, CGDisplayCopyDisplayMode, CGDisplayMode, CGGetActiveDisplayList,
};

use super::{Connection, TargetWindow, check, ffi::*, owned};

pub struct Display {
    pub id: u32,
    pub bounds: CGRect,
    pub scale: f64,
    pub space: u64,
    pub fullscreen: bool,
}

pub struct Displays {
    pub screens: Vec<Display>,
}

impl Displays {
    pub fn read(connection: Connection) -> Result<Self> {
        let mut count = 0;
        unsafe {
            check(
                CGGetActiveDisplayList(0, std::ptr::null_mut(), &mut count).0,
                "count displays",
            )?;
        }
        let mut ids = vec![0; count as usize];
        unsafe {
            check(
                CGGetActiveDisplayList(count, ids.as_mut_ptr(), &mut count).0,
                "list displays",
            )?;
        }
        ids.truncate(count as usize);
        let screens = ids
            .into_iter()
            .map(|id| {
                let mode = CGDisplayCopyDisplayMode(id).context("read display mode")?;
                let scale = CGDisplayMode::pixel_width(Some(&mode)) as f64
                    / CGDisplayMode::width(Some(&mode)) as f64;
                unsafe {
                    let uuid = owned(CGDisplayCreateUUIDFromDisplayID(id), "get display UUID")?;
                    let uuid =
                        CFUUID::new_string(None, Some(&uuid)).context("format display UUID")?;
                    let space = SLSManagedDisplayGetCurrentSpace(connection.id, &uuid);
                    Ok(Display {
                        id,
                        bounds: CGDisplayBounds(id),
                        scale,
                        space,
                        fullscreen: SLSSpaceGetType(connection.id, space) == 4,
                    })
                }
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self { screens })
    }

    /// The display containing the largest visible portion chooses the scale.
    /// A fullscreen Space has no borders, including for sticky windows.
    pub fn for_window(&self, window: &TargetWindow) -> Option<&Display> {
        self.screens
            .iter()
            .filter(|display| {
                !display.fullscreen && (window.sticky || window.spaces.contains(&display.space))
            })
            .map(|display| (display, overlap_area(window.frame, display.bounds)))
            .filter(|(_, area)| *area > 0.0)
            .max_by(|(_, left), (_, right)| left.total_cmp(right))
            .map(|(display, _)| display)
    }
}

fn overlap_area(left: CGRect, right: CGRect) -> f64 {
    let width = (left.origin.x + left.size.width).min(right.origin.x + right.size.width)
        - left.origin.x.max(right.origin.x);
    let height = (left.origin.y + left.size.height).min(right.origin.y + right.size.height)
        - left.origin.y.max(right.origin.y);
    width.max(0.0) * height.max(0.0)
}
