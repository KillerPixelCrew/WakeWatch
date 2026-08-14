//! Process-owned power requests used by the manual tray-menu controls.

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Power::{
    POWER_REQUEST_TYPE, PowerClearRequest, PowerCreateRequest, PowerRequestDisplayRequired,
    PowerRequestSystemRequired, PowerSetRequest,
};
use windows::Win32::System::Threading::{
    POWER_REQUEST_CONTEXT_SIMPLE_STRING, REASON_CONTEXT, REASON_CONTEXT_0,
};
use windows::core::PWSTR;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WakelockKind {
    Display,
    Standby,
}

impl WakelockKind {
    fn request_type(self) -> POWER_REQUEST_TYPE {
        match self {
            Self::Display => PowerRequestDisplayRequired,
            Self::Standby => PowerRequestSystemRequired,
        }
    }

    fn reason(self) -> &'static str {
        match self {
            Self::Display => "Manually keeping the display awake in WakeWatch",
            Self::Standby => "Manually preventing standby in WakeWatch",
        }
    }
}

/// Owns an active Windows power request. Closing the handle also guarantees
/// that Windows releases the request if normal cleanup is interrupted.
struct PowerRequest {
    handle: HANDLE,
    kind: WakelockKind,
}

impl PowerRequest {
    fn acquire(kind: WakelockKind) -> windows::core::Result<Self> {
        let mut reason: Vec<u16> = kind.reason().encode_utf16().chain(Some(0)).collect();
        let context = REASON_CONTEXT {
            Version: 0,
            Flags: POWER_REQUEST_CONTEXT_SIMPLE_STRING,
            Reason: REASON_CONTEXT_0 {
                SimpleReasonString: PWSTR(reason.as_mut_ptr()),
            },
        };

        let handle = unsafe { PowerCreateRequest(&context) }?;
        if let Err(error) = unsafe { PowerSetRequest(handle, kind.request_type()) } {
            let _ = unsafe { CloseHandle(handle) };
            return Err(error);
        }

        Ok(Self { handle, kind })
    }
}

impl Drop for PowerRequest {
    fn drop(&mut self) {
        let _ = unsafe { PowerClearRequest(self.handle, self.kind.request_type()) };
        let _ = unsafe { CloseHandle(self.handle) };
    }
}

#[derive(Default)]
pub struct ManualWakelocks {
    display: Option<PowerRequest>,
    standby: Option<PowerRequest>,
}

impl ManualWakelocks {
    pub fn is_held(&self, kind: WakelockKind) -> bool {
        match kind {
            WakelockKind::Display => self.display.is_some(),
            WakelockKind::Standby => self.standby.is_some(),
        }
    }

    pub fn toggle(&mut self, kind: WakelockKind) -> windows::core::Result<()> {
        let slot = match kind {
            WakelockKind::Display => &mut self.display,
            WakelockKind::Standby => &mut self.standby,
        };

        if slot.is_some() {
            *slot = None;
        } else {
            *slot = Some(PowerRequest::acquire(kind)?);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_wakelocks_start_released() {
        let locks = ManualWakelocks::default();
        assert!(!locks.is_held(WakelockKind::Display));
        assert!(!locks.is_held(WakelockKind::Standby));
    }
}
