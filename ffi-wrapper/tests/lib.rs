//! Integration tests for nomos-da FFI wrapper

use kzgrs_backend::encoder::DaEncoderParams;
use nomos_da_ffi::{
    nomos_da_cleanup, nomos_da_encoder_encode, nomos_da_encoder_free,
    nomos_da_encoder_new, nomos_da_encoded_data_free,
    nomos_da_encoded_data_get_data, nomos_da_free_padded_data,
    nomos_da_init, nomos_da_pad_to_chunk_size, nomos_da_verifier_free,
    nomos_da_verifier_new, EncodedDataHandle, NomosDaResult,
};
use std::ptr;

const CHUNK_SIZE: usize = DaEncoderParams::MAX_BLS12_381_ENCODING_CHUNK_SIZE;

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
    
    let padded_len = if data_size % CHUNK_SIZE == 0 {
        data_size
    } else {
        data_size + (CHUNK_SIZE - (data_size % CHUNK_SIZE))
    };
    
    assert_eq!(encoded.data.len(), padded_len, "Encoded data length should be padded to {}", padded_len);
    assert_eq!(&encoded.data[..data_size], data.as_slice(), "Original data should be preserved at the beginning");
    
    for i in data_size..padded_len {
        assert_eq!(encoded.data[i], 0, "Padding byte at index {} should be zero", i);
    }
    assert!(!encoded.chunked_data.0.is_empty(), "chunked_data should not be empty");
    assert!(!encoded.extended_data.0.is_empty(), "extended_data should not be empty");
    assert!(!encoded.row_commitments.is_empty(), "row_commitments should not be empty");
    assert!(!encoded.combined_column_proofs.is_empty(), "combined_column_proofs should not be empty");

    let chunks_per_row = column_count / 2;
    let bytes_per_row = chunks_per_row * CHUNK_SIZE;
    let expected_rows = (padded_len + bytes_per_row - 1) / bytes_per_row;
    let expected_chunks = (padded_len + CHUNK_SIZE - 1) / CHUNK_SIZE;
    let expected_columns_after_rs = column_count;
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

    let mut out_data = vec![0u8; padded_len * 2];
    let mut out_len = out_data.len();
    let result = nomos_da_encoded_data_get_data(
        out_handle,
        out_data.as_mut_ptr(),
        &mut out_len,
    );
    assert_eq!(result, NomosDaResult::Success);
    assert_eq!(out_len, padded_len, "Retrieved data length should match padded length");
    assert_eq!(&out_data[..data_size], data.as_slice(), "Original data should be preserved at the beginning");
    for i in data_size..padded_len {
        assert_eq!(out_data[i], 0, "Padding byte at index {} should be zero", i);
    }

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

unsafe fn test_padding(data_size: usize) {
    let data = create_test_data(data_size);

    let mut out_data: *mut u8 = ptr::null_mut();
    let mut out_len: usize = 0;

    let result = nomos_da_pad_to_chunk_size(
        data.as_ptr(),
        data_size,
        &mut out_data,
        &mut out_len,
    );

    assert_eq!(result, NomosDaResult::Success, "Padding should succeed for size {}", data_size);
    assert!(!out_data.is_null(), "Output data pointer should not be null");
    
    let expected_padded_len = if data_size % CHUNK_SIZE == 0 {
        data_size
    } else {
        data_size + (CHUNK_SIZE - (data_size % CHUNK_SIZE))
    };
    
    assert_eq!(out_len, expected_padded_len, "Padded length should be {} for input size {}", expected_padded_len, data_size);
    assert_eq!(out_len % CHUNK_SIZE, 0, "Padded length should be a multiple of {}", CHUNK_SIZE);

    let padded_slice = std::slice::from_raw_parts(out_data, out_len);
    
    assert_eq!(&padded_slice[..data_size], data.as_slice(), "Original data should be preserved at the beginning");
    
    for i in data_size..out_len {
        assert_eq!(padded_slice[i], 0, "Padding byte at index {} should be zero", i);
    }

    nomos_da_free_padded_data(out_data, out_len);
}

unsafe fn test_padding_failure(data_size: usize) {
    let data = if data_size > 0 {
        create_test_data(data_size)
    } else {
        Vec::new()
    };

    let mut out_data: *mut u8 = ptr::null_mut();
    let mut out_len: usize = 0;

    let result = nomos_da_pad_to_chunk_size(
        if data_size > 0 { data.as_ptr() } else { ptr::null() },
        data_size,
        &mut out_data,
        &mut out_len,
    );

    assert_ne!(result, NomosDaResult::Success, "Padding should fail for size {}", data_size);
    assert!(out_data.is_null(), "Output data pointer should be null on failure");
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
fn test_encode_size_less_than_chunk() {
    unsafe { test_encode_success(15, 4); }
}

#[test]
fn test_encode_size_more_than_chunk_not_multiple() {
    unsafe {
        test_encode_success(32, 4);
        test_encode_success(50, 4);
        test_encode_success(60, 4);
    }
}

#[test]
fn test_encode_size_one_chunk() {
    unsafe { test_encode_success(CHUNK_SIZE, 4); }
}

#[test]
fn test_encode_size_2_chunks() {
    unsafe { test_encode_success(2 * CHUNK_SIZE, 4); }
}

#[test]
fn test_encode_size_10_chunks() {
    unsafe { test_encode_success(10 * CHUNK_SIZE, 4); }
}

#[test]
fn test_encode_column_count_2() {
    unsafe {
        test_encode_success(CHUNK_SIZE, 2);
        test_encode_success(2 * CHUNK_SIZE, 2);
        test_encode_success(4 * CHUNK_SIZE, 2);
    }
}

#[test]
fn test_encode_column_count_8() {
    unsafe {
        test_encode_success(CHUNK_SIZE, 8);
        test_encode_success(4 * CHUNK_SIZE, 8);
        test_encode_success(8 * CHUNK_SIZE, 8);
    }
}

#[test]
fn test_pad_size_0() {
    unsafe { test_padding_failure(0); }
}

#[test]
fn test_pad_size_less_than_chunk() {
    unsafe { test_padding(15); }
}

#[test]
fn test_pad_size_one_chunk() {
    unsafe { test_padding(CHUNK_SIZE); }
}

#[test]
fn test_pad_size_between_chunk_and_2_chunks() {
    unsafe { test_padding(45); }
}
