//! FFI wrapper for nomos-da Rust library

use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;
use std::sync::Mutex;

use kzgrs::KzgRsError;
use kzgrs_backend::{
    common::share::DaShare,
    encoder::{DaEncoder, DaEncoderParams, EncodedData},
    kzg_keys::VERIFICATION_KEY,
    verifier::DaVerifier,
};
use nomos_core::da::{blob::Share as _, DaEncoder as _};

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

/// Opaque handle for a share
#[repr(C)]
pub struct ShareHandle {
    pub share: DaShare,
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
        if encoder.is_null() {
            set_error(format!("Encoder handle is null (data_len: {})", data_len));
        } else if data.is_null() {
            set_error(format!("Data pointer is null (data_len: {})", data_len));
        } else {
            set_error(format!("Output handle is null (data_len: {})", data_len));
        }
        return NomosDaResult::ErrorInvalidInput;
    }

    let mut padded_data_ptr: *mut u8 = ptr::null_mut();
    let mut padded_len: CSizeT = 0;

    let pad_result = nomos_da_pad_to_chunk_size(
        data,
        data_len,
        &mut padded_data_ptr,
        &mut padded_len,
    );

    if pad_result != NomosDaResult::Success {
        return pad_result;
    }

    let chunk_size = DaEncoderParams::MAX_BLS12_381_ENCODING_CHUNK_SIZE;
    let padded_slice = std::slice::from_raw_parts(padded_data_ptr, padded_len);
    let result = match (*encoder).encoder.encode(padded_slice) {
        Ok(encoded) => {
            *out_handle = Box::into_raw(Box::new(EncodedDataHandle { data: encoded }));
            NomosDaResult::Success
        }
        Err(e) => {
            set_error(format!(
                "Encoding error: {:?} (data_len: {}, padded_len: {}, chunk_size: {})",
                e, data_len, padded_len, chunk_size
            ));
            NomosDaResult::ErrorInternal
        }
    };

    nomos_da_free_padded_data(padded_data_ptr, padded_len);
    result
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

#[no_mangle]
pub unsafe extern "C" fn nomos_da_pad_to_chunk_size(
    data: *const u8,
    data_len: CSizeT,
    out_data: *mut *mut u8,
    out_len: *mut CSizeT,
) -> NomosDaResult {
    if out_data.is_null() || out_len.is_null() {
        let chunk_size = DaEncoderParams::MAX_BLS12_381_ENCODING_CHUNK_SIZE;
        if out_data.is_null() {
            set_error(format!("Output data pointer is null (data_len: {}, chunk_size: {})", data_len, chunk_size));
        } else {
            set_error(format!("Output length pointer is null (data_len: {}, chunk_size: {})", data_len, chunk_size));
        }
        return NomosDaResult::ErrorInvalidInput;
    }

    let chunk_size = DaEncoderParams::MAX_BLS12_381_ENCODING_CHUNK_SIZE;
    
    if data_len == 0 {
        set_error(format!(
            "Data length must be greater than 0, got {} (chunk_size: {})",
            data_len, chunk_size
        ));
        return NomosDaResult::ErrorInvalidInput;
    }

    if data.is_null() {
        set_error(format!(
            "Data pointer is null (data_len: {}, chunk_size: {})",
            data_len, chunk_size
        ));
        return NomosDaResult::ErrorInvalidInput;
    }
    let padding_needed = if data_len % chunk_size == 0 {
        0
    } else {
        chunk_size - (data_len % chunk_size)
    };
    let padded_len = data_len + padding_needed;

    let mut padded = vec![0u8; padded_len];
    if data_len > 0 {
        ptr::copy_nonoverlapping(
            data,
            padded.as_mut_ptr(),
            data_len,
        );
    }

    let boxed = padded.into_boxed_slice();
    let ptr = Box::into_raw(boxed) as *mut u8;
    
    *out_data = ptr;
    *out_len = padded_len;

    NomosDaResult::Success
}

#[no_mangle]
pub unsafe extern "C" fn nomos_da_free_padded_data(data: *mut u8, len: CSizeT) {
    if !data.is_null() && len > 0 {
        let slice_ptr: *mut [u8] = ptr::slice_from_raw_parts_mut(data, len);
        let _ = Box::from_raw(slice_ptr);
    }
}

#[no_mangle]
pub unsafe extern "C" fn nomos_da_encoded_data_get_share_count(
    handle: *mut EncodedDataHandle,
) -> CSizeT {
    if handle.is_null() {
        return 0;
    }
    (*handle).data.combined_column_proofs.len()
}

#[no_mangle]
pub unsafe extern "C" fn nomos_da_encoded_data_get_share(
    handle: *mut EncodedDataHandle,
    index: CSizeT,
    out_share_handle: *mut *mut ShareHandle,
) -> NomosDaResult {
    if handle.is_null() || out_share_handle.is_null() {
        if handle.is_null() {
            set_error(format!("EncodedData handle is null (share_index: {})", index));
        } else {
            set_error(format!("Output share handle is null (share_index: {})", index));
        }
        return NomosDaResult::ErrorInvalidInput;
    }

    match (*handle).data.to_da_share(index) {
        Some(share) => {
            *out_share_handle = Box::into_raw(Box::new(ShareHandle { share }));
            NomosDaResult::Success
        }
        None => {
            let share_count = (*handle).data.combined_column_proofs.len();
            set_error(format!(
                "Share index {} is out of bounds. Valid range: 0..{} (share_count: {})",
                index, share_count, share_count
            ));
            NomosDaResult::ErrorInvalidInput
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn nomos_da_share_free(handle: *mut ShareHandle) {
    if !handle.is_null() {
        let _ = Box::from_raw(handle);
    }
}

#[no_mangle]
pub unsafe extern "C" fn nomos_da_verifier_verify(
    verifier: *mut VerifierHandle,
    share_handle: *mut ShareHandle,
    rows_domain_size: CSizeT,
) -> bool {
    if verifier.is_null() || share_handle.is_null() {
        if verifier.is_null() {
            set_error(format!(
                "Verifier handle is null (rows_domain_size: {})",
                rows_domain_size
            ));
        } else {
            set_error(format!(
                "Share handle is null (rows_domain_size: {})",
                rows_domain_size
            ));
        }
        return false;
    }

    if rows_domain_size == 0 {
        set_error(format!(
            "Rows domain size must be greater than 0, got {}",
            rows_domain_size
        ));
        return false;
    }

    let share = &(*share_handle).share;
    let (light_share, commitments) = share.clone().into_share_and_commitments();
    
    let is_valid = (*verifier).verifier.verify(&light_share, &commitments, rows_domain_size);
    
    if !is_valid {
        set_error(format!(
            "Share verification failed (share_idx: {}, rows_domain_size: {})",
            light_share.share_idx, rows_domain_size
        ));
    }
    
    is_valid
}
