//! System audio capture — records everything the computer plays.
//!
//! Platform backends behind one interface so call mode can capture remote
//! participants without any third-party audio driver:
//!
//! - macOS 14.4+: CoreAudio Process Tap (`AudioHardwareCreateProcessTap`)
//!   wrapped in a private aggregate device we open with an IO block. One-time
//!   TCC permission ("Screen & System Audio Recording") is prompted by the OS
//!   on first use.
//! - Linux: PulseAudio/PipeWire sink `.monitor` source via libpulse-simple.
//! - Anything else: unavailable — callers fall back to the loopback-device
//!   path already present in the worker.

use anyhow::Result;

/// What a successful capture reports back to the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureMethod {
    /// macOS CoreAudio Process Tap
    SystemTap,
    /// Linux PulseAudio/PipeWire sink monitor
    PulseMonitor,
}

impl CaptureMethod {
    pub fn label(&self) -> &'static str {
        match self {
            CaptureMethod::SystemTap => "system tap (macOS)",
            CaptureMethod::PulseMonitor => "monitor (Linux)",
        }
    }
}

/// A live system-audio capture. Dropping it stops capture and releases
/// system resources (tap + aggregate device on macOS, stream on Linux).
pub enum SystemAudioCapture {
    #[cfg(target_os = "macos")]
    Tap(super::system_audio::macos::TapCapture),
    #[cfg(all(target_os = "linux", feature = "pulse-capture"))]
    Pulse(super::system_audio::linux::PulseCapture),
}

impl SystemAudioCapture {
    /// Start capturing the system output mix. Sample format is f32 at the
    /// system rate; samples are pushed into the shared buffer as they arrive.
    pub fn start(
        push: std::sync::Arc<std::sync::Mutex<Vec<f32>>>,
        rate_out: std::sync::Arc<std::sync::Mutex<u32>>,
    ) -> Result<(Self, u32, CaptureMethod)> {
        #[cfg(target_os = "macos")]
        {
            let (cap, rate) = super::system_audio::macos::TapCapture::start(push, rate_out)?;
            Ok((Self::Tap(cap), rate, CaptureMethod::SystemTap))
        }
        #[cfg(all(target_os = "linux", feature = "pulse-capture"))]
        {
            let (cap, rate) = super::system_audio::linux::PulseCapture::start(push, rate_out)?;
            Ok((Self::Pulse(cap), rate, CaptureMethod::PulseMonitor))
        }
        #[cfg(not(any(
            target_os = "macos",
            all(target_os = "linux", feature = "pulse-capture")
        )))]
        {
            let _ = (push, rate_out);
            Err(anyhow::anyhow!(
                "system audio capture not supported on this platform"
            ))
        }
    }
}

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(all(target_os = "linux", feature = "pulse-capture"))]
pub mod linux;

#[cfg(not(any(
    target_os = "macos",
    all(target_os = "linux", feature = "pulse-capture")
)))]
pub mod null {
    //! Stub backend for unsupported platforms.
    pub struct StubCapture;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capture_method_labels() {
        assert_eq!(CaptureMethod::SystemTap.label(), "system tap (macOS)");
        assert_eq!(CaptureMethod::PulseMonitor.label(), "monitor (Linux)");
    }
}
