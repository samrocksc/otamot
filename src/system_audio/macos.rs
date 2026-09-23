//! macOS CoreAudio Process Tap capture (macOS 14.4+).
//!
//! Flow (mirrors Apple's AudioCap sample and screenpipe's production impl):
//!   1. `CATapDescription` — global mono tap that *excludes* our own process
//!      so Otamot never records its own playback.
//!   2. `AudioHardwareCreateProcessTap` → tap AudioObjectID.
//!   3. Wrap the tap in a **private aggregate device** (invisible to other
//!      processes, usable by us).
//!   4. Open the aggregate with an `AudioDeviceIOBlock` that pushes incoming
//!      f32 samples into the shared buffer.
//!
//! First use triggers the OS TCC prompt ("Screen & System Audio Recording").
//! Until approved, tap creation fails and the worker falls back to the
//! loopback-device path.

use anyhow::{anyhow, Result};
use block2::RcBlock;
use dispatch2::{DispatchQueue, DispatchQueueAttr};
use objc2::rc::Retained;
use objc2::AnyThread;
use objc2_core_audio::{
    kAudioAggregateDeviceIsPrivateKey, kAudioAggregateDeviceNameKey,
    kAudioAggregateDeviceTapAutoStartKey, kAudioAggregateDeviceTapListKey,
    kAudioAggregateDeviceUIDKey, kAudioDevicePropertyNominalSampleRate,
    kAudioHardwarePropertyTranslatePIDToProcessObject, kAudioObjectPropertyElementMain,
    kAudioObjectPropertyScopeGlobal, kAudioObjectSystemObject, kAudioSubTapUIDKey,
    kAudioTapPropertyUID, AudioDeviceCreateIOProcIDWithBlock, AudioDeviceDestroyIOProcID,
    AudioDeviceIOProcID, AudioDeviceStart, AudioDeviceStop, AudioHardwareCreateAggregateDevice,
    AudioHardwareCreateProcessTap, AudioHardwareDestroyAggregateDevice,
    AudioHardwareDestroyProcessTap, AudioObjectGetPropertyData, AudioObjectGetPropertyDataSize,
    AudioObjectID, AudioObjectPropertyAddress, CATapDescription, CATapMuteBehavior,
};
use objc2_core_audio_types::{AudioBufferList, AudioTimeStamp};
use objc2_core_foundation::{CFArray, CFBoolean, CFDictionary, CFRetained, CFString, CFType};
use objc2_foundation::{NSArray, NSNumber, NSUUID};
use std::ptr::NonNull;

use std::ffi::{c_void, CStr};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

const STATUS_OK: i32 = 0;
const NO_ERR: i32 = 0;
/// Aggregate device backing our tap; private so it never shows up in other
/// apps' device lists (and must never be enumerated as our input — that
/// would loop our own capture back in).
const TAP_AGGREGATE_NAME: &str = "OtamotSystemTap";

/// Our own process audio object id, resolved once (0 = unresolved).
static SELF_PROCESS_OBJECT_ID: AtomicU32 = AtomicU32::new(0);

/// Convert a static C-string key constant to a str (trailing NUL stripped).
fn key_str(key: &'static CStr) -> &'static str {
    let bytes = key.to_bytes();
    unsafe { std::str::from_utf8_unchecked(bytes) }
}

pub struct TapCapture {
    aggregate_id: AudioObjectID,
    tap_id: AudioObjectID,
    io_proc_id: AudioDeviceIOProcID,
    #[allow(dead_code)] // keeps the dispatch queue alive while IO runs
    queue_retainer: dispatch2::DispatchRetained<DispatchQueue>,
    #[allow(dead_code)] // future: read for UI "tap live" state
    started: Arc<AtomicBool>,
}

impl TapCapture {
    /// Create the tap + private aggregate device and start IO. Returns the
    /// handle and the aggregate's sample rate.
    /// Create the tap on a **detached thread**. `AudioDeviceStart` blocks
    /// indefinitely until the TCC "Screen & System Audio Recording" grant is
    /// in place — for ad-hoc signed binaries the prompt rarely surfaces, so
    /// callers get a channel they can poll (or ignore) instead of a hang.
    pub fn start_detached(
        push: Arc<Mutex<Vec<f32>>>,
        rate_out: Arc<std::sync::Mutex<u32>>,
        done_tx: std::sync::mpsc::Sender<Result<u32>>,
    ) {
        std::thread::spawn(move || {
            let result = unsafe { create_tap_capture(push, rate_out) };
            let _ = done_tx.send(result.map(|(_, rate)| rate));
        });
    }

    pub fn start(push: Arc<Mutex<Vec<f32>>>, rate_out: Arc<Mutex<u32>>) -> Result<(Self, u32)> {
        unsafe { create_tap_capture(push, rate_out) }
    }
}

unsafe fn create_tap_capture(
    push: Arc<Mutex<Vec<f32>>>,
    rate_out: Arc<Mutex<u32>>,
) -> Result<(TapCapture, u32)> {
    // --- 1. Tap description: global mono tap excluding our own process ---
    let self_pid_object = resolve_self_process_object_id();
    let exclude: Vec<Retained<NSNumber>> = if self_pid_object != 0 {
        vec![NSNumber::new_u32(self_pid_object)]
    } else {
        Vec::new()
    };
    let exclude_array = NSArray::from_retained_slice(&exclude);
    eprintln!(
        "[otamot] creating CATapDescription (global mono, excluding pid {})…",
        self_pid_object
    );
    let desc_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        CATapDescription::initMonoGlobalTapButExcludeProcesses(
            CATapDescription::alloc(),
            &exclude_array,
        )
    }));
    let desc = match desc_result {
        Ok(d) => {
            eprintln!("[otamot] CATapDescription created OK");
            d
        }
        Err(e) => {
            eprintln!(
                "[otamot] CATapDescription init PANICKED (ObjC exception): {:?}",
                e
            );
            return Err(anyhow!(
                "CATapDescription init threw an ObjC exception — \
                macOS version or entitlement mismatch"
            ));
        }
    };
    let () = objc2::msg_send![&desc, setPrivate: true];
    let () = objc2::msg_send![&desc, setMuteBehavior: CATapMuteBehavior::Unmuted];
    // A UUID on the tap description is REQUIRED for aggregate-device tap
    // lists to reference it (kAudioSubTapUIDKey must match).
    let desc_uuid = NSUUID::new();
    let () = objc2::msg_send![&desc, setUUID: desc_uuid.as_ref() as &objc2_foundation::NSUUID];

    // --- 2. Create the tap ---
    let mut tap_id: AudioObjectID = 0;
    eprintln!("[otamot] calling AudioHardwareCreateProcessTap…");
    eprintln!("[otamot] calling AudioHardwareCreateProcessTap…");
    let status = AudioHardwareCreateProcessTap(Some(&desc), &mut tap_id);
    eprintln!(
        "[otamot] AudioHardwareCreateProcessTap returned {:#x} (tap_id {})",
        status, tap_id
    );
    eprintln!(
        "[otamot] AudioHardwareCreateProcessTap returned {} (tap_id {})",
        status, tap_id
    );
    if status != STATUS_OK {
        eprintln!(
            "[otamot] AudioHardwareCreateProcessTap failed (OSStatus {:#x}). \
             First use: approve Otamot in System Settings > Privacy & Security \
             > Screen & System Audio Recording, then retry. If it never appears, \
             add target/debug/otamot manually.",
            status
        );
        return Err(anyhow!(
            "AudioHardwareCreateProcessTap failed (OSStatus {}). First use: \
             approve Otamot in System Settings > Privacy & Security > Screen \
             & System Audio Recording, then retry.",
            status
        ));
    }

    // --- 3. Private aggregate device wrapping the tap ---
    eprintln!("[otamot] step 3a: reading tap UID…");
    let tap_uid_string = match tap_uid(tap_id) {
        Ok(uid) => uid,
        Err(e) => {
            let _ = AudioHardwareDestroyProcessTap(tap_id);
            return Err(e.context("reading tap UID"));
        }
    };

    eprintln!("[otamot] step 3b: tap UID read OK, building dicts…");
    let uuid_string = NSUUID::new().UUIDString().to_string();
    let uid_value = CFString::from_str(&format!("otamot-tap-{}", uuid_string));
    let name_value = CFString::from_str(TAP_AGGREGATE_NAME);
    let tap_uid_value = CFString::from_str(&tap_uid_string);

    // "taps": [ { "uid": <tapUID> } ]
    let sub_tap_uid_key = CFString::from_str(key_str(kAudioSubTapUIDKey));
    let tap_entry =
        CFDictionary::<CFString, CFString>::from_slices(&[&sub_tap_uid_key], &[&tap_uid_value]);
    let taps_array =
        CFArray::<CFDictionary<CFString, CFString>>::from_objects(&[tap_entry.as_ref()]);

    eprintln!("[otamot] step 3c: dict values built, creating CFDictionary…");
    let aggregate_dict = {
        let uid_cftype = CFRetained::<CFType>::from(CFRetained::<CFString>::from(&uid_value));
        let name_cftype = CFRetained::<CFType>::from(CFRetained::<CFString>::from(&name_value));
        let taps_cftype = CFRetained::<CFType>::from(CFRetained::<CFArray>::from(&taps_array));
        let true_cftype =
            CFRetained::<CFType>::from(CFRetained::<CFBoolean>::from(CFBoolean::new(true)));
        CFDictionary::<CFString, CFType>::from_slices(
            &[
                CFString::from_str(key_str(kAudioAggregateDeviceUIDKey)).as_ref(),
                CFString::from_str(key_str(kAudioAggregateDeviceNameKey)).as_ref(),
                CFString::from_str(key_str(kAudioAggregateDeviceIsPrivateKey)).as_ref(),
                CFString::from_str(key_str(kAudioAggregateDeviceTapListKey)).as_ref(),
                CFString::from_str(key_str(kAudioAggregateDeviceTapAutoStartKey)).as_ref(),
            ],
            &[
                &uid_cftype,
                &name_cftype,
                &true_cftype,
                &taps_cftype,
                &true_cftype,
            ],
        )
    };

    eprintln!("[otamot] step 3d: calling AudioHardwareCreateAggregateDevice…");
    let mut aggregate_id: AudioObjectID = 0;
    eprintln!("[otamot] creating private aggregate device (TCC grant may be pending here)…");
    let status = AudioHardwareCreateAggregateDevice(
        aggregate_dict.as_ref(),
        NonNull::new(std::ptr::from_mut(&mut aggregate_id)).unwrap(),
    );
    eprintln!(
        "[otamot] AudioHardwareCreateAggregateDevice returned {:#x} (aggregate_id {})",
        status, aggregate_id
    );
    if status != STATUS_OK {
        let _ = AudioHardwareDestroyProcessTap(tap_id);
        return Err(anyhow!(
            "AudioHardwareCreateAggregateDevice failed ({})",
            status
        ));
    }

    // --- 4. IO block feeding the shared buffer ---
    eprintln!("[otamot] step 4a: creating IO block…");
    let started = Arc::new(AtomicBool::new(false));
    let started_clone = Arc::clone(&started);
    let io_block = RcBlock::new(
        move |_now: NonNull<AudioTimeStamp>,
              input: NonNull<AudioBufferList>,
              _in_time: NonNull<AudioTimeStamp>,
              _output: NonNull<AudioBufferList>,
              _out_time: NonNull<AudioTimeStamp>| {
            started_clone.store(true, Ordering::Relaxed);
            let list = unsafe { &*input.as_ptr() };
            let mut collected: Vec<f32> = Vec::new();
            for i in 0..list.mNumberBuffers as usize {
                let buf = &list.mBuffers[i];
                if buf.mData.is_null() || buf.mDataByteSize == 0 {
                    continue;
                }
                let frames = buf.mDataByteSize as usize / std::mem::size_of::<f32>();
                let samples = std::slice::from_raw_parts(buf.mData as *const f32, frames);
                collected.extend_from_slice(samples);
            }
            if !collected.is_empty() {
                if let Ok(mut p) = push.lock() {
                    p.extend_from_slice(&collected);
                }
            }
        },
    );

    eprintln!("[otamot] step 4b: creating dispatch queue…");
    let queue = DispatchQueue::new("otamot.tap.io", DispatchQueueAttr::SERIAL);

    eprintln!("[otamot] step 4c: calling AudioDeviceCreateIOProcIDWithBlock…");
    let mut io_proc_id: AudioDeviceIOProcID = None;
    let status = AudioDeviceCreateIOProcIDWithBlock(
        NonNull::new(std::ptr::from_mut(&mut io_proc_id)).unwrap(),
        aggregate_id,
        Some(&queue),
        RcBlock::as_ptr(&io_block),
    );
    if status != STATUS_OK {
        eprintln!(
            "[otamot] AudioDeviceCreateIOProcIDWithBlock failed (OSStatus {:#x})",
            status
        );
        let _ = AudioHardwareDestroyAggregateDevice(aggregate_id);
        let _ = AudioHardwareDestroyProcessTap(tap_id);
        return Err(anyhow!(
            "AudioDeviceCreateIOProcIDWithBlock failed ({})",
            status
        ));
    }

    eprintln!("[otamot] step 4d: calling AudioDeviceStart…");
    let status = AudioDeviceStart(aggregate_id, io_proc_id);
    if status != STATUS_OK {
        eprintln!("[otamot] AudioDeviceStart failed (OSStatus {:#x})", status);
        AudioDeviceDestroyIOProcID(aggregate_id, io_proc_id);
        let _ = AudioHardwareDestroyAggregateDevice(aggregate_id);
        let _ = AudioHardwareDestroyProcessTap(tap_id);
        return Err(anyhow!("AudioDeviceStart failed ({})", status));
    }

    // Aggregate sample rate: needed by the worker's resampler
    let rate = aggregate_sample_rate(aggregate_id).unwrap_or(48_000);
    *rate_out.lock().unwrap() = rate;

    Ok((
        TapCapture {
            aggregate_id,
            tap_id,
            io_proc_id,
            queue_retainer: queue,
            started,
        },
        rate,
    ))
}

impl Drop for TapCapture {
    fn drop(&mut self) {
        unsafe {
            let _ = AudioDeviceStop(self.aggregate_id, self.io_proc_id);
            AudioDeviceDestroyIOProcID(self.aggregate_id, self.io_proc_id);
            let _ = AudioHardwareDestroyAggregateDevice(self.aggregate_id);
            let _ = AudioHardwareDestroyProcessTap(self.tap_id);
        }
    }
}

/// Read the tap's persistent UID (a CFString) via kAudioTapPropertyUID.
unsafe fn tap_uid(tap_id: AudioObjectID) -> Result<String> {
    let mut address = AudioObjectPropertyAddress {
        mSelector: kAudioTapPropertyUID,
        mScope: kAudioObjectPropertyScopeGlobal,
        mElement: kAudioObjectPropertyElementMain,
    };
    let mut size: u32 = 0;
    let status = AudioObjectGetPropertyDataSize(
        tap_id,
        NonNull::new(std::ptr::from_mut(&mut address)).unwrap(),
        0,
        std::ptr::null(),
        NonNull::new(std::ptr::from_mut(&mut size)).unwrap(),
    );
    if status != NO_ERR || size < std::mem::size_of::<*const c_void>() as u32 {
        return Err(anyhow!("tap UID size query failed ({})", status));
    }
    let mut cf_string_ptr: *const c_void = std::ptr::null_mut();
    let status = AudioObjectGetPropertyData(
        tap_id,
        NonNull::new(std::ptr::from_mut(&mut address)).unwrap(),
        0,
        std::ptr::null(),
        NonNull::new(std::ptr::from_mut(&mut size)).unwrap(),
        NonNull::new(std::ptr::from_mut(&mut cf_string_ptr).cast::<c_void>()).unwrap(),
    );
    if status != NO_ERR || cf_string_ptr.is_null() {
        return Err(anyhow!("tap UID read failed ({})", status));
    }
    // CFString is toll-free bridged with NSString — use Display via NSString
    let ns_string: &objc2_foundation::NSString =
        &*(cf_string_ptr as *const objc2_foundation::NSString);
    Ok(ns_string.to_string())
}

/// Query the aggregate device's nominal sample rate.
unsafe fn aggregate_sample_rate(aggregate_id: AudioObjectID) -> Option<u32> {
    let mut address = AudioObjectPropertyAddress {
        mSelector: kAudioDevicePropertyNominalSampleRate,
        mScope: kAudioObjectPropertyScopeGlobal,
        mElement: kAudioObjectPropertyElementMain,
    };
    let mut rate: f64 = 0.0;
    let mut size = std::mem::size_of::<f64>() as u32;
    let status = AudioObjectGetPropertyData(
        aggregate_id,
        NonNull::new(std::ptr::from_mut(&mut address)).unwrap(),
        0,
        std::ptr::null(),
        NonNull::new(std::ptr::from_mut(&mut size)).unwrap(),
        NonNull::new(std::ptr::from_mut(&mut rate).cast::<c_void>()).unwrap(),
    );
    if status == NO_ERR && rate > 0.0 {
        Some(rate as u32)
    } else {
        None
    }
}

/// Resolve our own pid to an AudioObjectID for the tap exclusion list.
unsafe fn resolve_self_process_object_id() -> AudioObjectID {
    let cached = SELF_PROCESS_OBJECT_ID.load(Ordering::Relaxed);
    if cached != 0 {
        return cached;
    }
    let pid: u32 = std::process::id();
    let mut address = AudioObjectPropertyAddress {
        mSelector: kAudioHardwarePropertyTranslatePIDToProcessObject,
        mScope: kAudioObjectPropertyScopeGlobal,
        mElement: kAudioObjectPropertyElementMain,
    };
    let mut object_id: AudioObjectID = 0;
    let mut size = std::mem::size_of::<AudioObjectID>() as u32;
    let status = AudioObjectGetPropertyData(
        kAudioObjectSystemObject as AudioObjectID,
        NonNull::new(std::ptr::from_mut(&mut address)).unwrap(),
        std::mem::size_of::<u32>() as u32,
        std::ptr::from_ref(&pid).cast::<c_void>(),
        NonNull::new(std::ptr::from_mut(&mut size)).unwrap(),
        NonNull::new(std::ptr::from_mut(&mut object_id).cast::<c_void>()).unwrap(),
    );
    if status == NO_ERR && object_id != 0 {
        SELF_PROCESS_OBJECT_ID.store(object_id, Ordering::Relaxed);
        object_id
    } else {
        0
    }
}
