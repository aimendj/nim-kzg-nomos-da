//! Integration tests for nomos-da FFI wrapper

use nomos_da_ffi::{
    nomos_da_cleanup, nomos_da_encoder_encode, nomos_da_encoder_free,
    nomos_da_encoder_new, nomos_da_encoded_data_free,
    nomos_da_encoded_data_get_data, nomos_da_init, nomos_da_verifier_free,
    nomos_da_verifier_new, EncodedDataHandle, NomosDaResult,
};
use std::ptr;

const CHUNK_SIZE: usize = 31;

fn create_test_data(size: usize) -> Vec<u8> {
    (1..=size).map(|i| (i % 256) as u8).collect()
}

unsafe fn test_encode_success(data_size: usize, column_count: usize) {
    let encoder = nomos_da_encoder_new(column_count);
    assert!(!encoder.is_null(), "Encoder should be created");

    let data = create_test_data(data_size);
    let mut out_handle: *mut EncodedDataHandle = ptr::null_mut();
    let result = nomos_da_encoder_encode(
        encoder,
        data.as_ptr(),
        data.len(),
        &mut out_handle,
    );
    
    assert_eq!(result, NomosDaResult::Success, "Encoding should succeed for size {} with column_count {}", data_size, column_count);
    assert!(!out_handle.is_null(), "Output handle should not be null");

    let encoded = &(*out_handle).data;
    assert_eq!(encoded.data, data, "Original data should match");
    assert!(!encoded.chunked_data.0.is_empty(), "chunked_data should not be empty");
    assert!(!encoded.extended_data.0.is_empty(), "extended_data should not be empty");
    assert!(!encoded.row_commitments.is_empty(), "row_commitments should not be empty");
    assert!(!encoded.combined_column_proofs.is_empty(), "combined_column_proofs should not be empty");

    let chunks_per_row = column_count / 2;
    let bytes_per_row = chunks_per_row * CHUNK_SIZE;
    let expected_rows = (data_size + bytes_per_row - 1) / bytes_per_row;
    let expected_chunks = (data_size + CHUNK_SIZE - 1) / CHUNK_SIZE;
    let expected_columns_after_rs = column_count;

    assert_eq!(encoded.data.len(), data_size, "Encoded data length should match input");
    assert_eq!(encoded.chunked_data.0.len(), expected_rows, "Number of rows should match expected");
    assert_eq!(encoded.extended_data.0.len(), expected_rows, "Extended data should have same number of rows");
    assert_eq!(encoded.row_commitments.len(), expected_rows, "Row commitments should match number of rows");

    let actual_columns = encoded.extended_data.0[0].0.len();
    assert_eq!(actual_columns, expected_columns_after_rs, "After RS encoding, each row should have {} columns", expected_columns_after_rs);
    
    for (i, row) in encoded.extended_data.0.iter().enumerate() {
        assert_eq!(row.0.len(), expected_columns_after_rs, "Row {} should have {} columns", i, expected_columns_after_rs);
    }

    assert_eq!(encoded.combined_column_proofs.len(), expected_columns_after_rs, "Number of column proofs should match number of columns");

    let mut total_chunks = 0;
    for row in encoded.chunked_data.0.iter() {
        assert!(row.0.len() <= chunks_per_row, "Each row should have at most {} chunks", chunks_per_row);
        total_chunks += row.0.len();
    }
    assert_eq!(total_chunks, expected_chunks, "Total chunks should match expected");

    let mut total_extended_chunks = 0;
    for row in encoded.extended_data.0.iter() {
        total_extended_chunks += row.0.len();
    }
    assert_eq!(total_extended_chunks, expected_rows * expected_columns_after_rs, "Total extended chunks should be rows × columns");

    let mut out_data = vec![0u8; data_size * 2];
    let mut out_len = out_data.len();
    let result = nomos_da_encoded_data_get_data(
        out_handle,
        out_data.as_mut_ptr(),
        &mut out_len,
    );
    assert_eq!(result, NomosDaResult::Success);
    assert_eq!(&out_data[..out_len], data.as_slice(), "Retrieved data should match original");

    nomos_da_encoded_data_free(out_handle);
    nomos_da_encoder_free(encoder);
}

unsafe fn test_encode_failure(data_size: usize, column_count: usize) {
    let encoder = nomos_da_encoder_new(column_count);
    assert!(!encoder.is_null(), "Encoder should be created");

    let data = create_test_data(data_size);
    let mut out_handle: *mut EncodedDataHandle = ptr::null_mut();
    let result = nomos_da_encoder_encode(
        encoder,
        data.as_ptr(),
        data.len(),
        &mut out_handle,
    );
    
    assert_ne!(result, NomosDaResult::Success, "Encoding should fail for size {} with column_count {}", data_size, column_count);
    assert!(out_handle.is_null(), "Output handle should be null on failure");

    nomos_da_encoder_free(encoder);
}

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
fn test_encode_size_0() {
    unsafe { test_encode_failure(0, 4); }
}

#[test]
fn test_encode_size_less_than_31() {
    unsafe { test_encode_failure(15, 4); }
}

#[test]
fn test_encode_size_more_than_31_not_multiple() {
    unsafe {
        test_encode_failure(32, 4);
        test_encode_failure(50, 4);
        test_encode_failure(60, 4);
    }
}

#[test]
fn test_encode_size_31() {
    unsafe { test_encode_success(31, 4); }
}

#[test]
fn test_encode_size_2_times_31() {
    unsafe { test_encode_success(2 * 31, 4); }
}

#[test]
fn test_encode_size_10_times_31() {
    unsafe { test_encode_success(10 * 31, 4); }
}

#[test]
fn test_encode_column_count_2() {
    unsafe {
        test_encode_success(31, 2);
        test_encode_success(62, 2);
        test_encode_success(124, 2);
    }
}

#[test]
fn test_encode_column_count_8() {
    unsafe {
        test_encode_success(31, 8);
        test_encode_success(124, 8);
        test_encode_success(248, 8);
    }
}
