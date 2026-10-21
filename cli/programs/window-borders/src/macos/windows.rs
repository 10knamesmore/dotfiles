use anyhow::{Context, Result};
use objc2_core_foundation::{
    CFArray, CFDictionary, CFNumber, CFRetained, CFString, CFType, CGRect,
};
use objc2_core_graphics::{CGWindowListCopyWindowInfo, CGWindowListOption, kCGWindowNumber};

use super::{Connection, TargetWindow, check, displays::Displays, ffi::*, owned, window_array};

const DOCUMENT: u64 = 1 << 0;
const FLOATING: u64 = 1 << 1;
const ATTACHED: u64 = 1 << 7;
const STICKY: u64 = 1 << 11;
const IGNORES_CYCLE: u64 = 1 << 18;
const MODAL: u64 = 1 << 31;

struct WindowQuery {
    // The result iterator is retained independently from the query.
    iterator: CFRetained<CFType>,
}

struct QueriedWindow {
    id: WindowId,
    tags: u64,
    level: i32,
    eligible: bool,
}

impl WindowQuery {
    fn new(connection: Connection, ids: &CFArray) -> Result<Self> {
        unsafe {
            let query = owned(
                SLSWindowQueryWindows(connection.id, ids, 0),
                "query windows",
            )?;
            let iterator = owned(
                SLSWindowQueryResultCopyWindows(&query),
                "create window iterator",
            )?;
            Ok(Self { iterator })
        }
    }

    fn next(&self) -> Option<QueriedWindow> {
        unsafe {
            if SLSWindowIteratorAdvance(&self.iterator) {
                let tags = SLSWindowIteratorGetTags(&self.iterator);
                let attributes = SLSWindowIteratorGetAttributes(&self.iterator);
                let parent = SLSWindowIteratorGetParentID(&self.iterator);
                // Root document windows and floating modal windows, not menus,
                // attached sheets, desktop elements, or our own border windows.
                let eligible = parent == 0
                    && (attributes & 2 != 0 || tags & 0x400000000000000 != 0)
                    && tags & (ATTACHED | IGNORES_CYCLE) == 0
                    && (tags & DOCUMENT != 0 || tags & (FLOATING | MODAL) == (FLOATING | MODAL));
                return Some(QueriedWindow {
                    id: SLSWindowIteratorGetWindowID(&self.iterator),
                    tags,
                    level: SLSWindowIteratorGetLevel(&self.iterator),
                    eligible,
                });
            }
        }
        None
    }

    fn next_eligible(&self) -> Option<QueriedWindow> {
        while let Some(window) = self.next() {
            if window.eligible {
                return Some(window);
            }
        }
        None
    }
}

impl Connection {
    pub fn discover_windows(self) -> Result<Vec<WindowId>> {
        let info = CGWindowListCopyWindowInfo(CGWindowListOption::ExcludeDesktopElements, 0)
            .context("list desktop windows")?;
        // CGWindowListCopyWindowInfo specifies an array of window dictionaries.
        let info: CFRetained<CFArray<CFDictionary<CFString, CFType>>> =
            unsafe { CFRetained::cast_unchecked(info) };
        let ids = info
            .iter()
            .map(|window| {
                let number = window
                    .get(unsafe { kCGWindowNumber })
                    .context("window ID missing")?;
                let number = number
                    .downcast::<CFNumber>()
                    .map_err(|_| anyhow::anyhow!("window ID is not a number"))?;
                Ok(number.as_i32().context("invalid window ID")? as u32)
            })
            .collect::<Result<Vec<_>>>()?;
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let query = WindowQuery::new(self, window_array(&ids).as_opaque())?;
        let mut windows = Vec::new();
        while let Some(window) = query.next_eligible() {
            windows.push(window.id);
        }
        Ok(windows)
    }

    /// A window can disappear between its notification and this snapshot.
    pub fn target(self, window: WindowId) -> Result<Option<TargetWindow>> {
        let ids = window_array(&[window]);
        let query = WindowQuery::new(self, ids.as_opaque())?;
        let Some(description) = query.next() else {
            return Ok(None);
        };
        let mut frame = CGRect::default();
        let mut shown = 0;
        unsafe {
            let result = SLSGetWindowBounds(self.id, window, &mut frame);
            if result != 0 {
                let mut owner = 0;
                if SLSGetWindowOwner(self.id, window, &mut owner) != 0 {
                    return Ok(None);
                }
                check(result, "get target bounds")?;
            }
            check(
                SLSWindowIsOrderedIn(self.id, window, &mut shown),
                "get target visibility",
            )?;
            let spaces = owned(
                SLSCopySpacesForWindows(self.id, 7, ids.as_opaque()),
                "copy target spaces",
            )?;
            let spaces = spaces
                .iter()
                .map(|space| {
                    space
                        .as_i64()
                        .map(|id| id as u64)
                        .context("invalid Space ID")
                })
                .collect::<Result<Vec<_>>>()?;
            let fullscreen = description.tags & STICKY == 0
                && spaces
                    .iter()
                    .any(|&space| SLSSpaceGetType(self.id, space) == 4);
            Ok(Some(TargetWindow {
                frame,
                level: description.level,
                sublevel: SLSGetWindowSubLevel(self.id, window),
                shown: shown != 0,
                spaces,
                sticky: description.tags & STICKY != 0,
                fullscreen,
                eligible: description.eligible,
            }))
        }
    }

    pub fn focused_window(self, displays: &Displays) -> Result<Option<WindowId>> {
        if displays.screens.is_empty() {
            return Ok(None);
        }
        let mut process = ProcessSerialNumber::default();
        let mut owner = 0;
        unsafe {
            check(
                _SLPSGetFrontProcess(&mut process),
                "get foreground application",
            )?;
            check(
                SLSGetConnectionIDForPSN(self.id, &process, &mut owner),
                "get foreground connection",
            )?;
            let spaces = CFArray::from_retained_objects(
                &displays
                    .screens
                    .iter()
                    .map(|display| CFNumber::new_i64(display.space as i64))
                    .collect::<Vec<_>>(),
            );
            let windows = owned(
                SLSCopyWindowsWithOptionsAndTags(self.id, owner, spaces.as_opaque(), 2, &0, &0),
                "list foreground windows",
            )?;
            if windows.is_empty() {
                return Ok(None);
            }
            Ok(WindowQuery::new(self, windows.as_opaque())?
                .next_eligible()
                .map(|window| window.id))
        }
    }

    pub fn watch_windows(self, windows: &[WindowId]) -> Result<()> {
        unsafe {
            check(
                SLSRequestNotificationsForWindows(
                    self.id,
                    windows.as_ptr(),
                    windows.len().try_into()?,
                ),
                "subscribe to windows",
            )
        }
    }
}
