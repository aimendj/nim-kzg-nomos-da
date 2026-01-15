//! Integration tests for nomos-da FFI wrapper

use nomos_da_ffi::{
    nomos_da_cleanup, nomos_da_encoder_encode, nomos_da_encoder_free,
    nomos_da_encoder_new, nomos_da_encoded_data_free,
    nomos_da_encoded_data_get_data, nomos_da_init, nomos_da_verifier_free,
    nomos_da_verifier_new, EncodedDataHandle, NomosDaResult,
};
use std::ptr;

#[test]
fn test_init_cleanup() {
    assert_eq!(nomos_da_init() as i32, NomosDaResult::Success as i32);
    nomos_da_cleanup();
}

#[test]
fn test_encoder_create_and_free() {
    unsafe {
        let encoder = nomos_da_encoder_new(4);
        assert!(!encoder.is_null());
        nomos_da_encoder_free(encoder);
    }
}

#[test]
fn test_verifier_create_and_free() {
    unsafe {
        let verifier = nomos_da_verifier_new();
        assert!(!verifier.is_null());
        nomos_da_verifier_free(verifier);
    }
}

#[test]
fn test_encode_simple() {
    unsafe {
        let encoder = nomos_da_encoder_new(4);
        assert!(!encoder.is_null());

        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31];
        let mut out_handle: *mut EncodedDataHandle = ptr::null_mut();
        let result = nomos_da_encoder_encode(
            encoder,
            data.as_ptr(),
            data.len(),
            &mut out_handle,
        );
        assert_eq!(result, NomosDaResult::Success);
        assert!(!out_handle.is_null());

        let mut out_data = vec![0u8; 1024];
        let mut out_len = out_data.len();
        let result = nomos_da_encoded_data_get_data(
            out_handle,
            out_data.as_mut_ptr(),
            &mut out_len,
        );
        assert_eq!(result, NomosDaResult::Success);
        assert_eq!(&out_data[..out_len], data.as_slice());

        nomos_da_encoded_data_free(out_handle);
        nomos_da_encoder_free(encoder);
    }
}
