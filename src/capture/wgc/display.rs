use std::ffi::CStr;
use std::fmt;
use std::mem::size_of;

use windows::Graphics::Capture::GraphicsCaptureItem;
use windows::Win32::Devices::Display::{
    DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
    DISPLAYCONFIG_DEVICE_INFO_HEADER, DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE,
    DISPLAYCONFIG_SOURCE_DEVICE_NAME, DISPLAYCONFIG_TARGET_DEVICE_NAME, DisplayConfigGetDeviceInfo,
    GetDisplayConfigBufferSizes, QDC_ONLY_ACTIVE_PATHS, QueryDisplayConfig,
};
use windows::Win32::Foundation::{LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoA, HDC, HMONITOR, MONITORINFO, MONITORINFOEXA,
};
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::core::BOOL;

use crate::capture::DisplayInfo;
use anyhow::Result;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Display {
    pub handle: HMONITOR,
    pub name: String,
}

// SAFETY: HMONITOR is a system handle that is safe to send across threads.
unsafe impl Send for Display {}

impl Display {
    pub fn online() -> Result<Vec<Self>> {
        unsafe {
            let displays = Box::into_raw(Box::default());
            let _ = EnumDisplayMonitors(None, None, Some(enum_monitor), LPARAM(displays as isize));
            Ok(*Box::from_raw(displays))
        }
    }

    pub fn new(handle: HMONITOR) -> Result<Self> {
        Ok(Self {
            handle,
            name: unsafe { get_display_name(handle) },
        })
    }

    pub fn select(&self) -> Result<GraphicsCaptureItem> {
        let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
        Ok(unsafe { interop.CreateForMonitor(self.handle) }?)
    }

    /// Returns the DPI scale factor for this monitor (e.g. 1.0, 1.25, 1.5, 2.0).
    pub fn dpi_scale(&self) -> f64 {
        use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
        let mut dpi_x = 0u32;
        let mut dpi_y = 0u32;
        unsafe {
            let _ = GetDpiForMonitor(self.handle, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y);
        }
        if dpi_x > 0 { dpi_x as f64 / 96.0 } else { 1.0 }
    }

    /// Returns (width, height, x, y) of the monitor in physical pixels.
    pub fn rect(&self) -> (f32, f32, f32, f32) {
        unsafe {
            let mut info = MONITORINFO {
                cbSize: size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            let _ = GetMonitorInfoA(self.handle, &mut info);
            let rc = info.rcMonitor;
            (
                (rc.right - rc.left) as f32,
                (rc.bottom - rc.top) as f32,
                rc.left as f32,
                rc.top as f32,
            )
        }
    }
}

unsafe fn get_display_name(handle: HMONITOR) -> String {
    unsafe {
        let (device_name, width, height) = {
            let info = MONITORINFOEXA {
                monitorInfo: MONITORINFO {
                    cbSize: size_of::<MONITORINFOEXA>() as u32,
                    ..Default::default()
                },
                szDevice: [0; 32],
            };
            let _ = GetMonitorInfoA(handle, &info as *const _ as *mut _);
            (
                CStr::from_ptr(info.szDevice.as_ptr() as _)
                    .to_str()
                    .unwrap()
                    .to_string(),
                info.monitorInfo.rcMonitor.right - info.monitorInfo.rcMonitor.left,
                info.monitorInfo.rcMonitor.bottom - info.monitorInfo.rcMonitor.top,
            )
        };
        let name = try_get_user_friendly_name(device_name.clone()).unwrap_or(device_name);

        format!("{} ({} x {})", name, width, height)
    }
}

unsafe fn try_get_user_friendly_name(device_name: String) -> Option<String> {
    unsafe {
        let mut num_path_array_elements = 0;
        let mut num_mode_info_array_elements = 0;
        let _ = GetDisplayConfigBufferSizes(
            QDC_ONLY_ACTIVE_PATHS,
            &mut num_path_array_elements,
            &mut num_mode_info_array_elements,
        );

        let mut path_info_array = vec![Default::default(); num_path_array_elements as usize];
        let mut mode_info_array = vec![Default::default(); num_mode_info_array_elements as usize];
        let _ = QueryDisplayConfig(
            QDC_ONLY_ACTIVE_PATHS,
            &mut num_path_array_elements,
            path_info_array.as_mut_ptr(),
            &mut num_mode_info_array_elements,
            mode_info_array.as_mut_ptr(),
            None,
        );

        mode_info_array
            .iter()
            .filter(|source_mode| source_mode.infoType == DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE)
            .find_map(|source_mode| {
                let source_device_name = DISPLAYCONFIG_SOURCE_DEVICE_NAME {
                    header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                        adapterId: source_mode.adapterId,
                        id: source_mode.id,
                        size: size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32,
                        r#type: DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
                    },
                    ..Default::default()
                };
                let _ =
                    DisplayConfigGetDeviceInfo(&source_device_name.header as *const _ as *mut _);
                let gdi_device_name = widestring::U16CString::from_ptr_str(
                    source_device_name.viewGdiDeviceName.as_ptr(),
                )
                .to_string()
                .ok()?;

                if gdi_device_name == device_name {
                    let target_mode = {
                        let id = path_info_array
                            .iter()
                            .find(|path_info| path_info.sourceInfo.id == source_mode.id)?
                            .targetInfo
                            .id;
                        mode_info_array
                            .iter()
                            .find(|target_mode| target_mode.id == id)?
                    };
                    let target_device_name = DISPLAYCONFIG_TARGET_DEVICE_NAME {
                        header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                            adapterId: target_mode.adapterId,
                            id: target_mode.id,
                            size: size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32,
                            r#type: DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
                        },
                        ..Default::default()
                    };
                    let _ = DisplayConfigGetDeviceInfo(
                        &target_device_name.header as *const _ as *mut _,
                    );
                    let user_friendly_name = widestring::U16CString::from_ptr_str(
                        target_device_name.monitorFriendlyDeviceName.as_ptr(),
                    )
                    .to_string()
                    .ok()?;

                    if user_friendly_name.is_empty() {
                        None
                    } else {
                        Some(user_friendly_name)
                    }
                } else {
                    None
                }
            })
    }
}

impl fmt::Display for Display {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

// callback function for EnumDisplayMonitors
extern "system" fn enum_monitor(monitor: HMONITOR, _: HDC, _: *mut RECT, state: LPARAM) -> BOOL {
    unsafe {
        // get the vector from the param, use leak because this function is not responsible for its lifetime
        let state = Box::leak(Box::from_raw(state.0 as *mut Vec<Display>));
        state.push(Display::new(monitor).unwrap());
    }
    true.into()
}

impl DisplayInfo for GraphicsCaptureItem {
    fn resolution(&self) -> (u32, u32) {
        (
            self.Size().unwrap().Width as u32,
            self.Size().unwrap().Height as u32,
        )
    }
    fn dpi_conversion_factor(&self) -> f64 {
        1.0
    }
}
