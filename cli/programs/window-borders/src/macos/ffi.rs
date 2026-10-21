//! Private SkyLight entry points for macOS 15.

use objc2_core_foundation::{
    CFArray, CFDictionary, CFMachPort, CFNumber, CFString, CFType, CFUUID, CGPoint, CGRect,
};
use objc2_core_graphics::{CGContext, CGEvent};
use std::ffi::c_void;

pub type ConnectionId = i32;
pub type WindowId = u32;
pub type NotifyCallback = unsafe extern "C" fn(u32, *const c_void, usize, *mut c_void);

#[repr(C)]
#[derive(Default)]
pub struct ProcessSerialNumber {
    pub high: u32,
    pub low: u32,
}

unsafe extern "C" {
    pub fn SLSMainConnectionID() -> ConnectionId;
    pub fn CGSNewRegionWithRect(rect: *const CGRect, region: *mut *mut CFType) -> i32;
    pub fn SLSNewWindow(
        cid: ConnectionId,
        backing: i32,
        x: f32,
        y: f32,
        region: &CFType,
        wid: *mut WindowId,
    ) -> i32;
    pub fn SLSReleaseWindow(cid: ConnectionId, wid: WindowId) -> i32;
    pub fn SLSSetWindowResolution(cid: ConnectionId, wid: WindowId, resolution: f64) -> i32;
    pub fn SLSSetWindowTags(cid: ConnectionId, wid: WindowId, tags: *const u64, size: i32) -> i32;
    pub fn SLSClearWindowTags(cid: ConnectionId, wid: WindowId, tags: *const u64, size: i32)
    -> i32;
    pub fn SLSSetWindowOpacity(cid: ConnectionId, wid: WindowId, opaque: bool) -> i32;
    pub fn SLSWindowSetShadowProperties(wid: WindowId, properties: &CFDictionary) -> i32;
    pub fn SLWindowContextCreate(
        cid: ConnectionId,
        wid: WindowId,
        options: *const CFDictionary,
    ) -> *mut CGContext;
    pub fn SLSFlushWindowContentRegion(
        cid: ConnectionId,
        wid: WindowId,
        dirty: *const c_void,
    ) -> i32;
    pub fn SLSGetWindowBounds(cid: ConnectionId, wid: WindowId, bounds: *mut CGRect) -> i32;
    pub fn SLSGetWindowSubLevel(cid: ConnectionId, wid: WindowId) -> i32;
    pub fn SLSGetWindowOwner(cid: ConnectionId, wid: WindowId, owner: *mut ConnectionId) -> i32;
    pub fn SLSWindowIsOrderedIn(cid: ConnectionId, wid: WindowId, shown: *mut u8) -> i32;
    pub fn SLSCopySpacesForWindows(
        cid: ConnectionId,
        selector: i32,
        windows: &CFArray,
    ) -> *mut CFArray<CFNumber>;
    pub fn SLSMoveWindowsToManagedSpace(cid: ConnectionId, windows: &CFArray, space: u64);
    pub fn SLSWindowQueryWindows(cid: ConnectionId, windows: &CFArray, options: u32)
    -> *mut CFType;
    pub fn SLSWindowQueryResultCopyWindows(query: &CFType) -> *mut CFType;
    pub fn SLSWindowIteratorAdvance(iterator: &CFType) -> bool;
    pub fn SLSWindowIteratorGetWindowID(iterator: &CFType) -> WindowId;
    pub fn SLSWindowIteratorGetParentID(iterator: &CFType) -> WindowId;
    pub fn SLSWindowIteratorGetTags(iterator: &CFType) -> u64;
    pub fn SLSWindowIteratorGetAttributes(iterator: &CFType) -> u64;
    pub fn SLSWindowIteratorGetLevel(iterator: &CFType) -> i32;
    pub fn SLSCopyWindowsWithOptionsAndTags(
        cid: ConnectionId,
        owner: ConnectionId,
        spaces: &CFArray,
        options: u32,
        set_tags: *const u64,
        clear_tags: *const u64,
    ) -> *mut CFArray<CFNumber>;
    pub fn _SLPSGetFrontProcess(process: *mut ProcessSerialNumber) -> i32;
    pub fn SLSGetConnectionIDForPSN(
        cid: ConnectionId,
        process: &ProcessSerialNumber,
        owner: *mut ConnectionId,
    ) -> i32;
    pub fn SLSManagedDisplayGetCurrentSpace(cid: ConnectionId, display: &CFString) -> u64;
    pub fn SLSSpaceGetType(cid: ConnectionId, space: u64) -> i32;
    pub fn CGDisplayCreateUUIDFromDisplayID(display: u32) -> *mut CFUUID;
    pub fn SLSTransactionCreate(cid: ConnectionId) -> *mut CFType;
    // These setters encode commands, not CGError results (macOS 15.3.1).
    pub fn SLSTransactionMoveWindowWithGroup(transaction: &CFType, wid: WindowId, point: CGPoint);
    pub fn SLSTransactionSetWindowLevel(transaction: &CFType, wid: WindowId, level: i32);
    pub fn SLSTransactionSetWindowSubLevel(transaction: &CFType, wid: WindowId, level: i32);
    pub fn SLSTransactionOrderWindow(
        transaction: &CFType,
        wid: WindowId,
        order: i32,
        relative_to: WindowId,
    );
    pub fn SLSTransactionCommit(transaction: &CFType, synchronous: i32);
    pub fn SLSRegisterNotifyProc(callback: NotifyCallback, event: u32, context: *mut c_void)
    -> i32;
    pub fn SLSRequestNotificationsForWindows(
        cid: ConnectionId,
        windows: *const WindowId,
        count: i32,
    ) -> i32;
    pub fn SLSGetEventPort(cid: ConnectionId, port: *mut u32) -> i32;
    pub fn SLEventCreateNextEvent(cid: ConnectionId) -> *mut CGEvent;
    pub fn _CFMachPortSetOptions(port: &CFMachPort, options: i32);
}
