//! FFI wrapper for nomos-da Rust library

use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;
use std::sync::Mutex;

use kzgrs::KzgRsError;
use kzgrs_backend::{
    encoder::{DaEncoder, DaEncoderParams, EncodedData},
    kzg_keys::VERIFICATION_KEY,
    verifier::DaVerifier,
};
use nomos_core::da::DaEncoder as _;

pub type CSizeT = usize;

thread_local! {
    static LAST_ERROR: Mutex<Option<String>> = Mutex::new(None);
}

fn set_error(err: String) {
    LAST_ERROR.with(|e| *e.lock().unwrap() = Some(err));
}

fn take_error() -> Option<String> {
    LAST_ERROR.with(|e| e.lock().unwrap().take())
}

/// Result code for FFI operations
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NomosDaResult {
    Success = 0,
    ErrorInvalidInput = -1,
    ErrorInternal = -2,
    ErrorAllocation = -3,
}

impl From<Result<(), KzgRsError>> for NomosDaResult {
    fn from(result: Result<(), KzgRsError>) -> Self {
        match result {
            Ok(_) => NomosDaResult::Success,
            Err(e) => {
                set_error(format!("{:?}", e));
                NomosDaResult::ErrorInternal
            }
        }
    }
}

/// Opaque handle for an encoder
#[repr(C)]
pub struct EncoderHandle {
    encoder: DaEncoder,
}

/// Opaque handle for a verifier
#[repr(C)]
pub struct VerifierHandle {
    verifier: DaVerifier,
}

/// Opaque handle for encoded data
#[repr(C)]
pub struct EncodedDataHandle {
    pub data: EncodedData,
}

#[no_mangle]
pub unsafe extern "C" fn nomos_da_free_string(s: *mut c_char) {
    if !s.is_null() {
        let _ = CString::from_raw(s);
    }
}

#[no_mangle]
pub unsafe extern "C" fn nomos_da_get_last_error() -> *mut c_char {
    take_error()
        .and_then(|err| CString::new(err).ok())
        .map(|s| s.into_raw())
        .unwrap_or(ptr::null_mut())
}

#[no_mangle]
pub extern "C" fn nomos_da_init() -> NomosDaResult {
    NomosDaResult::Success
}

#[no_mangle]
pub extern "C" fn nomos_da_cleanup() {}

#[no_mangle]
pub unsafe extern "C" fn nomos_da_encoder_new(column_count: CSizeT) -> *mut EncoderHandle {
    let encoder = DaEncoder::new(DaEncoderParams::default_with(column_count));
    Box::into_raw(Box::new(EncoderHandle { encoder }))
}

#[no_mangle]
pub unsafe extern "C" fn nomos_da_encoder_free(handle: *mut EncoderHandle) {
    if !handle.is_null() {
        let _ = Box::from_raw(handle);
    }
}

#[no_mangle]
pub unsafe extern "C" fn nomos_da_encoder_encode(
    encoder: *mut EncoderHandle,
    data: *const u8,
    data_len: CSizeT,
    out_handle: *mut *mut EncodedDataHandle,
) -> NomosDaResult {
    if encoder.is_null() || data.is_null() || out_handle.is_null() {
        return NomosDaResult::ErrorInvalidInput;
    }

    if data_len == 0 || data_len % DaEncoderParams::MAX_BLS12_381_ENCODING_CHUNK_SIZE != 0 {
        set_error(format!(
            "Data length must be a multiple of {} bytes, got {}",
            DaEncoderParams::MAX_BLS12_381_ENCODING_CHUNK_SIZE,
            data_len
        ));
        return NomosDaResult::ErrorInvalidInput;
    }

    match (*encoder).encoder.encode(std::slice::from_raw_parts(data, data_len)) {
        Ok(encoded) => {
            *out_handle = Box::into_raw(Box::new(EncodedDataHandle { data: encoded }));
            NomosDaResult::Success
        }
        Err(e) => {
            set_error(format!("Encoding error: {:?}", e));
            NomosDaResult::ErrorInternal
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn nomos_da_encoded_data_free(handle: *mut EncodedDataHandle) {
    if !handle.is_null() {
        let _ = Box::from_raw(handle);
    }
}

#[no_mangle]
pub unsafe extern "C" fn nomos_da_encoded_data_get_data(
    handle: *mut EncodedDataHandle,
    out_data: *mut u8,
    out_len: *mut CSizeT,
) -> NomosDaResult {
    if handle.is_null() || out_data.is_null() || out_len.is_null() {
        return NomosDaResult::ErrorInvalidInput;
    }

    let data = &(*handle).data.data;
    let len = data.len();

    if *out_len < len {
        *out_len = len;
        return NomosDaResult::ErrorInvalidInput;
    }

    ptr::copy_nonoverlapping(data.as_ptr(), out_data, len);
    *out_len = len;
    NomosDaResult::Success
}

#[no_mangle]
pub unsafe extern "C" fn nomos_da_verifier_new() -> *mut VerifierHandle {
    Box::into_raw(Box::new(VerifierHandle {
        verifier: DaVerifier::new(VERIFICATION_KEY.clone()),
    }))
}

#[no_mangle]
pub unsafe extern "C" fn nomos_da_verifier_free(handle: *mut VerifierHandle) {
    if !handle.is_null() {
        let _ = Box::from_raw(handle);
    }
}

#[no_mangle]
pub extern "C" fn nomos_da_max_chunk_size() -> CSizeT {
    DaEncoderParams::MAX_BLS12_381_ENCODING_CHUNK_SIZE
}
