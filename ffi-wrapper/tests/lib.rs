//! Integration tests for nomos-da FFI wrapper

use kzgrs_backend::encoder::DaEncoderParams;
use nomos_da_ffi::{
    nomos_da_cleanup, nomos_da_encoder_encode, nomos_da_encoder_free,
    nomos_da_encoder_new, nomos_da_encoded_data_free,
    nomos_da_encoded_data_get_data, nomos_da_encoded_data_get_share,
    nomos_da_encoded_data_get_share_count, nomos_da_free_padded_data,
    nomos_da_init, nomos_da_pad_to_chunk_size, nomos_da_share_free,
    nomos_da_verifier_free, nomos_da_verifier_new, nomos_da_verifier_verify,
    EncodedDataHandle, NomosDaResult, ShareHandle,
};
use std::ptr;

// ============================================================================
// Constants and Helper Functions
// ============================================================================

const CHUNK_SIZE: usize = DaEncoderParams::MAX_BLS12_381_ENCODING_CHUNK_SIZE;

fn create_test_data(size: usize) -> Vec<u8> {
    (1..=size).map(|i| (i % 256) as u8).collect()
}

unsafe fn test_encode_success(data_size: usize, column_count: usize) {
    let encoder = nomos_da_encoder_new(column_count);
    assert!(!encoder.is_null(), "Encoder should be created (data_size: {}, column_count: {}, chunk_size: {})", data_size, column_count, CHUNK_SIZE);

    let data = create_test_data(data_size);
    let mut out_handle: *mut EncodedDataHandle = ptr::null_mut();
    let result = nomos_da_encoder_encode(
        encoder,
        data.as_ptr(),
        data.len(),
        &mut out_handle,
    );
    
    assert_eq!(result, NomosDaResult::Success, "Encoding should succeed (data_size: {}, column_count: {}, chunk_size: {})", data_size, column_count, CHUNK_SIZE);
    assert!(!out_handle.is_null(), "Output handle should not be null (data_size: {}, column_count: {}, chunk_size: {})", data_size, column_count, CHUNK_SIZE);

    let encoded = &(*out_handle).data;
    
    let padded_len = if data_size % CHUNK_SIZE == 0 {
        data_size
    } else {
        data_size + (CHUNK_SIZE - (data_size % CHUNK_SIZE))
    };
    
    assert_eq!(encoded.data.len(), padded_len, "Encoded data length should be padded to {} (data_size: {}, column_count: {}, chunk_size: {})", padded_len, data_size, column_count, CHUNK_SIZE);
    assert_eq!(&encoded.data[..data_size], data.as_slice(), "Original data should be preserved at the beginning (data_size: {}, column_count: {}, chunk_size: {})", data_size, column_count, CHUNK_SIZE);
    
    for i in data_size..padded_len {
        assert_eq!(encoded.data[i], 0, "Padding byte at index {} should be zero (data_size: {}, column_count: {}, chunk_size: {})", i, data_size, column_count, CHUNK_SIZE);
    }
    assert!(!encoded.chunked_data.0.is_empty(), "chunked_data should not be empty (data_size: {}, column_count: {}, chunk_size: {})", data_size, column_count, CHUNK_SIZE);
    assert!(!encoded.extended_data.0.is_empty(), "extended_data should not be empty (data_size: {}, column_count: {}, chunk_size: {})", data_size, column_count, CHUNK_SIZE);
    assert!(!encoded.row_commitments.is_empty(), "row_commitments should not be empty (data_size: {}, column_count: {}, chunk_size: {})", data_size, column_count, CHUNK_SIZE);
    assert!(!encoded.combined_column_proofs.is_empty(), "combined_column_proofs should not be empty (data_size: {}, column_count: {}, chunk_size: {})", data_size, column_count, CHUNK_SIZE);

    let chunks_per_row = column_count / 2;
    let bytes_per_row = chunks_per_row * CHUNK_SIZE;
    let expected_rows = (padded_len + bytes_per_row - 1) / bytes_per_row;
    let expected_chunks = (padded_len + CHUNK_SIZE - 1) / CHUNK_SIZE;
    let expected_columns_after_rs = column_count;
    assert_eq!(encoded.chunked_data.0.len(), expected_rows, "Number of rows should match expected (data_size: {}, column_count: {}, chunk_size: {}, expected_rows: {})", data_size, column_count, CHUNK_SIZE, expected_rows);
    assert_eq!(encoded.extended_data.0.len(), expected_rows, "Extended data should have same number of rows (data_size: {}, column_count: {}, chunk_size: {}, expected_rows: {})", data_size, column_count, CHUNK_SIZE, expected_rows);
    assert_eq!(encoded.row_commitments.len(), expected_rows, "Row commitments should match number of rows (data_size: {}, column_count: {}, chunk_size: {}, expected_rows: {})", data_size, column_count, CHUNK_SIZE, expected_rows);

    let actual_columns = encoded.extended_data.0[0].0.len();
    assert_eq!(actual_columns, expected_columns_after_rs, "After RS encoding, each row should have {} columns (data_size: {}, column_count: {}, chunk_size: {})", expected_columns_after_rs, data_size, column_count, CHUNK_SIZE);
    
    for (i, row) in encoded.extended_data.0.iter().enumerate() {
        assert_eq!(row.0.len(), expected_columns_after_rs, "Row {} should have {} columns (data_size: {}, column_count: {}, chunk_size: {})", i, expected_columns_after_rs, data_size, column_count, CHUNK_SIZE);
    }

    assert_eq!(encoded.combined_column_proofs.len(), expected_columns_after_rs, "Number of column proofs should match number of columns (data_size: {}, column_count: {}, chunk_size: {}, expected_columns: {})", data_size, column_count, CHUNK_SIZE, expected_columns_after_rs);

    let mut total_chunks = 0;
    for row in encoded.chunked_data.0.iter() {
        assert!(row.0.len() <= chunks_per_row, "Each row should have at most {} chunks (data_size: {}, column_count: {}, chunk_size: {})", chunks_per_row, data_size, column_count, CHUNK_SIZE);
        total_chunks += row.0.len();
    }
    assert_eq!(total_chunks, expected_chunks, "Total chunks should match expected (data_size: {}, column_count: {}, chunk_size: {}, expected_chunks: {})", data_size, column_count, CHUNK_SIZE, expected_chunks);

    let mut total_extended_chunks = 0;
    for row in encoded.extended_data.0.iter() {
        total_extended_chunks += row.0.len();
    }
    assert_eq!(total_extended_chunks, expected_rows * expected_columns_after_rs, "Total extended chunks should be rows × columns (data_size: {}, column_count: {}, chunk_size: {}, expected: {})", data_size, column_count, CHUNK_SIZE, expected_rows * expected_columns_after_rs);

    let mut out_data = vec![0u8; padded_len * 2];
    let mut out_len = out_data.len();
    let result = nomos_da_encoded_data_get_data(
        out_handle,
        out_data.as_mut_ptr(),
        &mut out_len,
    );
    assert_eq!(result, NomosDaResult::Success, "Get data should succeed (data_size: {}, column_count: {}, chunk_size: {})", data_size, column_count, CHUNK_SIZE);
    assert_eq!(out_len, padded_len, "Retrieved data length should match padded length (data_size: {}, column_count: {}, chunk_size: {}, padded_len: {})", data_size, column_count, CHUNK_SIZE, padded_len);
    assert_eq!(&out_data[..data_size], data.as_slice(), "Original data should be preserved at the beginning (data_size: {}, column_count: {}, chunk_size: {})", data_size, column_count, CHUNK_SIZE);
    for i in data_size..padded_len {
        assert_eq!(out_data[i], 0, "Padding byte at index {} should be zero (data_size: {}, column_count: {}, chunk_size: {})", i, data_size, column_count, CHUNK_SIZE);
    }

    nomos_da_encoded_data_free(out_handle);
    nomos_da_encoder_free(encoder);
}

unsafe fn test_encode_failure(data_size: usize, column_count: usize) {
    let encoder = nomos_da_encoder_new(column_count);
    assert!(!encoder.is_null(), "Encoder should be created (data_size: {}, column_count: {}, chunk_size: {})", data_size, column_count, CHUNK_SIZE);

    let data = create_test_data(data_size);
    let mut out_handle: *mut EncodedDataHandle = ptr::null_mut();
    let result = nomos_da_encoder_encode(
        encoder,
        data.as_ptr(),
        data.len(),
        &mut out_handle,
    );
    
    assert_ne!(result, NomosDaResult::Success, "Encoding should fail (data_size: {}, column_count: {}, chunk_size: {})", data_size, column_count, CHUNK_SIZE);
    assert!(out_handle.is_null(), "Output handle should be null on failure (data_size: {}, column_count: {}, chunk_size: {})", data_size, column_count, CHUNK_SIZE);

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

    assert_eq!(result, NomosDaResult::Success, "Padding should succeed (data_size: {}, chunk_size: {})", data_size, CHUNK_SIZE);
    assert!(!out_data.is_null(), "Output data pointer should not be null (data_size: {}, chunk_size: {})", data_size, CHUNK_SIZE);
    
    let expected_padded_len = if data_size % CHUNK_SIZE == 0 {
        data_size
    } else {
        data_size + (CHUNK_SIZE - (data_size % CHUNK_SIZE))
    };
    
    assert_eq!(out_len, expected_padded_len, "Padded length should be {} for input size {} (chunk_size: {})", expected_padded_len, data_size, CHUNK_SIZE);
    assert_eq!(out_len % CHUNK_SIZE, 0, "Padded length should be a multiple of {} (data_size: {}, chunk_size: {})", CHUNK_SIZE, data_size, CHUNK_SIZE);

    let padded_slice = std::slice::from_raw_parts(out_data, out_len);
    
    assert_eq!(&padded_slice[..data_size], data.as_slice(), "Original data should be preserved at the beginning (data_size: {}, chunk_size: {})", data_size, CHUNK_SIZE);
    
    for i in data_size..out_len {
        assert_eq!(padded_slice[i], 0, "Padding byte at index {} should be zero (data_size: {}, chunk_size: {})", i, data_size, CHUNK_SIZE);
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

    assert_ne!(result, NomosDaResult::Success, "Padding should fail (data_size: {}, chunk_size: {})", data_size, CHUNK_SIZE);
    assert!(out_data.is_null(), "Output data pointer should be null on failure (data_size: {}, chunk_size: {})", data_size, CHUNK_SIZE);
}

// ============================================================================
// Utility Tests
// ============================================================================

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

// ============================================================================
// Encoding Tests
// ============================================================================

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
fn test_encode_various_sizes_and_column_counts() {
    unsafe {
        let column_counts = [2, 4, 8];
        let chunk_multipliers = [1, 2, 4, 8];
        
        for column_count in column_counts.iter() {
            for multiplier in chunk_multipliers.iter() {
                let data_size = *multiplier * CHUNK_SIZE;
                test_encode_success(data_size, *column_count);
            }
        }
    }
}

// ============================================================================
// Padding Tests
// ============================================================================

#[test]
fn test_pad_size_0() {
    unsafe { test_padding_failure(0); }
}

#[test]
fn test_pad_various_sizes() {
    unsafe {
        let sizes = [
            15,                                    // less than chunk
            CHUNK_SIZE,                            // exactly one chunk
            45,                                    // between 1 and 2 chunks
            2 * CHUNK_SIZE,                        // exactly 2 chunks
        ];
        
        for size in sizes.iter() {
            test_padding(*size);
        }
    }
}

// ============================================================================
// Share Extraction Tests
// ============================================================================

#[test]
fn test_get_share_count() {
    unsafe {
        let encoder = nomos_da_encoder_new(4);
        assert!(!encoder.is_null());

        let data = create_test_data(CHUNK_SIZE);
        let mut out_handle: *mut EncodedDataHandle = ptr::null_mut();
        let result = nomos_da_encoder_encode(
            encoder,
            data.as_ptr(),
            data.len(),
            &mut out_handle,
        );
        assert_eq!(result, NomosDaResult::Success);
        assert!(!out_handle.is_null());

        let share_count = nomos_da_encoded_data_get_share_count(out_handle);
        assert_eq!(share_count, 4, "Share count should match column count (column_count: 4, chunk_size: {})", CHUNK_SIZE);

        nomos_da_encoded_data_free(out_handle);
        nomos_da_encoder_free(encoder);
    }
}

#[test]
fn test_get_share_valid_indices() {
    unsafe {
        let column_count = 4;
        let encoder = nomos_da_encoder_new(column_count);
        assert!(!encoder.is_null(), "Encoder should be created (column_count: {}, chunk_size: {})", column_count, CHUNK_SIZE);

        let data = create_test_data(CHUNK_SIZE);
        let mut out_handle: *mut EncodedDataHandle = ptr::null_mut();
        let result = nomos_da_encoder_encode(
            encoder,
            data.as_ptr(),
            data.len(),
            &mut out_handle,
        );
        assert_eq!(result, NomosDaResult::Success, "Encoding should succeed (column_count: {}, chunk_size: {})", column_count, CHUNK_SIZE);
        assert!(!out_handle.is_null(), "Output handle should not be null (column_count: {}, chunk_size: {})", column_count, CHUNK_SIZE);

        let share_count = nomos_da_encoded_data_get_share_count(out_handle);
        
        for i in 0..share_count {
            let mut share_handle: *mut ShareHandle = ptr::null_mut();
            let result = nomos_da_encoded_data_get_share(out_handle, i, &mut share_handle);
            assert_eq!(result, NomosDaResult::Success, "Should successfully get share {} (column_count: {}, chunk_size: {})", i, column_count, CHUNK_SIZE);
            assert!(!share_handle.is_null(), "Share handle should not be null for index {} (column_count: {}, chunk_size: {})", i, column_count, CHUNK_SIZE);
            
            let share = &(*share_handle).share;
            assert_eq!(share.share_idx, i as u16, "Share index should match (share_index: {}, column_count: {}, chunk_size: {})", i, column_count, CHUNK_SIZE);
            assert_eq!(share.rows_commitments.len(), 1, "Should have one row commitment (share_index: {}, column_count: {}, chunk_size: {})", i, column_count, CHUNK_SIZE);
            
            nomos_da_share_free(share_handle);
        }

        nomos_da_encoded_data_free(out_handle);
        nomos_da_encoder_free(encoder);
    }
}

#[test]
fn test_get_share_invalid_index() {
    unsafe {
        let column_count = 4;
        let encoder = nomos_da_encoder_new(column_count);
        assert!(!encoder.is_null(), "Encoder should be created (column_count: {}, chunk_size: {})", column_count, CHUNK_SIZE);

        let data = create_test_data(CHUNK_SIZE);
        let mut out_handle: *mut EncodedDataHandle = ptr::null_mut();
        let result = nomos_da_encoder_encode(
            encoder,
            data.as_ptr(),
            data.len(),
            &mut out_handle,
        );
        assert_eq!(result, NomosDaResult::Success, "Encoding should succeed (column_count: {}, chunk_size: {})", column_count, CHUNK_SIZE);
        assert!(!out_handle.is_null(), "Output handle should not be null (column_count: {}, chunk_size: {})", column_count, CHUNK_SIZE);

        let share_count = nomos_da_encoded_data_get_share_count(out_handle);
        let invalid_index = share_count;

        let mut share_handle: *mut ShareHandle = ptr::null_mut();
        let result = nomos_da_encoded_data_get_share(out_handle, invalid_index, &mut share_handle);
        assert_ne!(result, NomosDaResult::Success, "Should fail for invalid index (invalid_index: {}, share_count: {}, column_count: {}, chunk_size: {})", invalid_index, share_count, column_count, CHUNK_SIZE);
        assert!(share_handle.is_null(), "Share handle should be null on failure (invalid_index: {}, share_count: {}, column_count: {}, chunk_size: {})", invalid_index, share_count, column_count, CHUNK_SIZE);

        nomos_da_encoded_data_free(out_handle);
        nomos_da_encoder_free(encoder);
    }
}

#[test]
fn test_get_share_different_column_counts() {
    unsafe {
        for column_count in [2, 4, 8] {
            let encoder = nomos_da_encoder_new(column_count);
            assert!(!encoder.is_null(), "Encoder should be created (column_count: {}, chunk_size: {})", column_count, CHUNK_SIZE);

            let data = create_test_data(CHUNK_SIZE);
            let mut out_handle: *mut EncodedDataHandle = ptr::null_mut();
            let result = nomos_da_encoder_encode(
                encoder,
                data.as_ptr(),
                data.len(),
                &mut out_handle,
            );
            assert_eq!(result, NomosDaResult::Success, "Encoding should succeed (column_count: {}, chunk_size: {})", column_count, CHUNK_SIZE);

            let share_count = nomos_da_encoded_data_get_share_count(out_handle);
            assert_eq!(share_count, column_count, "Share count should match column count (column_count: {}, chunk_size: {}, share_count: {})", column_count, CHUNK_SIZE, share_count);

            for i in 0..share_count {
                let mut share_handle: *mut ShareHandle = ptr::null_mut();
                let result = nomos_da_encoded_data_get_share(out_handle, i, &mut share_handle);
                assert_eq!(result, NomosDaResult::Success, "Should successfully get share (share_index: {}, column_count: {}, chunk_size: {})", i, column_count, CHUNK_SIZE);
                assert!(!share_handle.is_null(), "Share handle should not be null (share_index: {}, column_count: {}, chunk_size: {})", i, column_count, CHUNK_SIZE);
                nomos_da_share_free(share_handle);
            }

            nomos_da_encoded_data_free(out_handle);
            nomos_da_encoder_free(encoder);
        }
    }
}

// ============================================================================
// Share Verification Tests
// ============================================================================

#[test]
fn test_verifier_verify_valid_shares() {
    unsafe {
        let column_count = 4;
        let encoder = nomos_da_encoder_new(column_count);
        assert!(!encoder.is_null(), "Encoder should be created (column_count: {}, chunk_size: {})", column_count, CHUNK_SIZE);

        let data = create_test_data(CHUNK_SIZE);
        let mut out_handle: *mut EncodedDataHandle = ptr::null_mut();
        let result = nomos_da_encoder_encode(
            encoder,
            data.as_ptr(),
            data.len(),
            &mut out_handle,
        );
        assert_eq!(result, NomosDaResult::Success, "Encoding should succeed (column_count: {}, chunk_size: {})", column_count, CHUNK_SIZE);
        assert!(!out_handle.is_null(), "Output handle should not be null (column_count: {}, chunk_size: {})", column_count, CHUNK_SIZE);

        let verifier = nomos_da_verifier_new();
        assert!(!verifier.is_null(), "Verifier should be created (column_count: {}, chunk_size: {})", column_count, CHUNK_SIZE);

        let share_count = nomos_da_encoded_data_get_share_count(out_handle);
        
        for i in 0..share_count {
            let mut share_handle: *mut ShareHandle = ptr::null_mut();
            let result = nomos_da_encoded_data_get_share(out_handle, i, &mut share_handle);
            assert_eq!(result, NomosDaResult::Success, "Should successfully get share (share_index: {}, column_count: {}, chunk_size: {})", i, column_count, CHUNK_SIZE);
            assert!(!share_handle.is_null(), "Share handle should not be null (share_index: {}, column_count: {}, chunk_size: {})", i, column_count, CHUNK_SIZE);

            let verify_result = nomos_da_verifier_verify(verifier, share_handle, column_count);
            assert!(verify_result, "Share verification should succeed (share_index: {}, column_count: {}, chunk_size: {})", i, column_count, CHUNK_SIZE);

            nomos_da_share_free(share_handle);
        }

        nomos_da_verifier_free(verifier);
        nomos_da_encoded_data_free(out_handle);
        nomos_da_encoder_free(encoder);
    }
}

#[test]
fn test_verifier_verify_different_column_counts() {
    unsafe {
        for column_count in [2, 4, 8] {
            let encoder = nomos_da_encoder_new(column_count);
            assert!(!encoder.is_null(), "Encoder should be created (column_count: {}, chunk_size: {})", column_count, CHUNK_SIZE);

            let data = create_test_data(CHUNK_SIZE);
            let mut out_handle: *mut EncodedDataHandle = ptr::null_mut();
            let result = nomos_da_encoder_encode(
                encoder,
                data.as_ptr(),
                data.len(),
                &mut out_handle,
            );
            assert_eq!(result, NomosDaResult::Success, "Encoding should succeed (column_count: {}, chunk_size: {})", column_count, CHUNK_SIZE);

            let verifier = nomos_da_verifier_new();
            assert!(!verifier.is_null(), "Verifier should be created (column_count: {}, chunk_size: {})", column_count, CHUNK_SIZE);

            let share_count = nomos_da_encoded_data_get_share_count(out_handle);
            
            for i in 0..share_count {
                let mut share_handle: *mut ShareHandle = ptr::null_mut();
                let result = nomos_da_encoded_data_get_share(out_handle, i, &mut share_handle);
                assert_eq!(result, NomosDaResult::Success, "Should successfully get share (share_index: {}, column_count: {}, chunk_size: {})", i, column_count, CHUNK_SIZE);
                
            let verify_result = nomos_da_verifier_verify(verifier, share_handle, column_count);
            assert!(verify_result, "Share verification should succeed (share_index: {}, column_count: {}, chunk_size: {})", i, column_count, CHUNK_SIZE);
                
                nomos_da_share_free(share_handle);
            }

            nomos_da_verifier_free(verifier);
            nomos_da_encoded_data_free(out_handle);
            nomos_da_encoder_free(encoder);
        }
    }
}

#[test]
fn test_verifier_verify_null_handles() {
    unsafe {
        let column_count = 4;
        let encoder = nomos_da_encoder_new(column_count);
        let data = create_test_data(CHUNK_SIZE);
        let mut out_handle: *mut EncodedDataHandle = ptr::null_mut();
        let result = nomos_da_encoder_encode(
            encoder,
            data.as_ptr(),
            data.len(),
            &mut out_handle,
        );
        assert_eq!(result, NomosDaResult::Success);

        let verifier = nomos_da_verifier_new();
        let mut share_handle: *mut ShareHandle = ptr::null_mut();
        let result = nomos_da_encoded_data_get_share(out_handle, 0, &mut share_handle);
        assert_eq!(result, NomosDaResult::Success);
        assert!(!share_handle.is_null());

        let verify_result_null_verifier = nomos_da_verifier_verify(ptr::null_mut(), share_handle, column_count);
        assert!(!verify_result_null_verifier, "Verification should fail with null verifier (column_count: {}, chunk_size: {})", column_count, CHUNK_SIZE);

        let verify_result_null_share = nomos_da_verifier_verify(verifier, ptr::null_mut(), column_count);
        assert!(!verify_result_null_share, "Verification should fail with null share (column_count: {}, chunk_size: {})", column_count, CHUNK_SIZE);

        let verify_result_invalid_domain = nomos_da_verifier_verify(verifier, share_handle, 0);
        assert!(!verify_result_invalid_domain, "Verification should fail with invalid domain size (column_count: {}, chunk_size: {})", column_count, CHUNK_SIZE);

        nomos_da_share_free(share_handle);
        nomos_da_verifier_free(verifier);
        nomos_da_encoded_data_free(out_handle);
        nomos_da_encoder_free(encoder);
    }
}

// ============================================================================
// Error Handling Tests
// ============================================================================

// Encoding Error Cases
#[test]
fn test_encode_null_encoder_handle() {
    unsafe {
        let data = create_test_data(CHUNK_SIZE);
        let mut out_handle: *mut EncodedDataHandle = ptr::null_mut();
        let result = nomos_da_encoder_encode(
            ptr::null_mut(),
            data.as_ptr(),
            data.len(),
            &mut out_handle,
        );
        assert_eq!(result, NomosDaResult::ErrorInvalidInput, "Should fail with null encoder handle");
        assert!(out_handle.is_null(), "Output handle should be null on failure");
    }
}

#[test]
fn test_encode_null_data_pointer() {
    unsafe {
        let encoder = nomos_da_encoder_new(4);
        assert!(!encoder.is_null());
        let mut out_handle: *mut EncodedDataHandle = ptr::null_mut();
        let result = nomos_da_encoder_encode(
            encoder,
            ptr::null(),
            10,
            &mut out_handle,
        );
        assert_eq!(result, NomosDaResult::ErrorInvalidInput, "Should fail with null data pointer");
        assert!(out_handle.is_null(), "Output handle should be null on failure");
        nomos_da_encoder_free(encoder);
    }
}

#[test]
fn test_encode_null_output_handle() {
    unsafe {
        let encoder = nomos_da_encoder_new(4);
        assert!(!encoder.is_null());
        let data = create_test_data(CHUNK_SIZE);
        let result = nomos_da_encoder_encode(
            encoder,
            data.as_ptr(),
            data.len(),
            ptr::null_mut(),
        );
        assert_eq!(result, NomosDaResult::ErrorInvalidInput, "Should fail with null output handle");
        nomos_da_encoder_free(encoder);
    }
}

// EncodedData Get Data Error Cases
#[test]
fn test_get_data_null_handle() {
    unsafe {
        let mut out_data = vec![0u8; 100];
        let mut out_len = out_data.len();
        let result = nomos_da_encoded_data_get_data(
            ptr::null_mut(),
            out_data.as_mut_ptr(),
            &mut out_len,
        );
        assert_eq!(result, NomosDaResult::ErrorInvalidInput, "Should fail with null handle");
    }
}

#[test]
fn test_get_data_null_out_data() {
    unsafe {
        let encoder = nomos_da_encoder_new(4);
        let data = create_test_data(CHUNK_SIZE);
        let mut out_handle: *mut EncodedDataHandle = ptr::null_mut();
        let result = nomos_da_encoder_encode(encoder, data.as_ptr(), data.len(), &mut out_handle);
        assert_eq!(result, NomosDaResult::Success);
        
        let mut out_len = 100;
        let result = nomos_da_encoded_data_get_data(
            out_handle,
            ptr::null_mut(),
            &mut out_len,
        );
        assert_eq!(result, NomosDaResult::ErrorInvalidInput, "Should fail with null out_data");
        
        nomos_da_encoded_data_free(out_handle);
        nomos_da_encoder_free(encoder);
    }
}

#[test]
fn test_get_data_null_out_len() {
    unsafe {
        let encoder = nomos_da_encoder_new(4);
        let data = create_test_data(CHUNK_SIZE);
        let mut out_handle: *mut EncodedDataHandle = ptr::null_mut();
        let result = nomos_da_encoder_encode(encoder, data.as_ptr(), data.len(), &mut out_handle);
        assert_eq!(result, NomosDaResult::Success);
        
        let mut out_data = vec![0u8; 100];
        let result = nomos_da_encoded_data_get_data(
            out_handle,
            out_data.as_mut_ptr(),
            ptr::null_mut(),
        );
        assert_eq!(result, NomosDaResult::ErrorInvalidInput, "Should fail with null out_len");
        
        nomos_da_encoded_data_free(out_handle);
        nomos_da_encoder_free(encoder);
    }
}

#[test]
fn test_get_data_buffer_too_small() {
    unsafe {
        let encoder = nomos_da_encoder_new(4);
        let data = create_test_data(CHUNK_SIZE);
        let mut out_handle: *mut EncodedDataHandle = ptr::null_mut();
        let result = nomos_da_encoder_encode(encoder, data.as_ptr(), data.len(), &mut out_handle);
        assert_eq!(result, NomosDaResult::Success);
        
        let mut out_data = vec![0u8; 1];
        let mut out_len = out_data.len();
        let result = nomos_da_encoded_data_get_data(
            out_handle,
            out_data.as_mut_ptr(),
            &mut out_len,
        );
        assert_eq!(result, NomosDaResult::ErrorInvalidInput, "Should fail with buffer too small");
        assert!(out_len > 1, "out_len should be updated to required size");
        
        nomos_da_encoded_data_free(out_handle);
        nomos_da_encoder_free(encoder);
    }
}

// Padding Error Cases
#[test]
fn test_pad_null_out_data() {
    unsafe {
        let data = create_test_data(15);
        let mut out_len: usize = 0;
        let result = nomos_da_pad_to_chunk_size(
            data.as_ptr(),
            data.len(),
            ptr::null_mut(),
            &mut out_len,
        );
        assert_eq!(result, NomosDaResult::ErrorInvalidInput, "Should fail with null out_data");
    }
}

#[test]
fn test_pad_null_out_len() {
    unsafe {
        let data = create_test_data(15);
        let mut out_data: *mut u8 = ptr::null_mut();
        let result = nomos_da_pad_to_chunk_size(
            data.as_ptr(),
            data.len(),
            &mut out_data,
            ptr::null_mut(),
        );
        assert_eq!(result, NomosDaResult::ErrorInvalidInput, "Should fail with null out_len");
        assert!(out_data.is_null(), "Output data should be null on failure");
    }
}

#[test]
fn test_pad_null_data_pointer() {
    unsafe {
        let mut out_data: *mut u8 = ptr::null_mut();
        let mut out_len: usize = 0;
        let result = nomos_da_pad_to_chunk_size(
            ptr::null(),
            10,
            &mut out_data,
            &mut out_len,
        );
        assert_eq!(result, NomosDaResult::ErrorInvalidInput, "Should fail with null data pointer (non-zero len)");
        assert!(out_data.is_null(), "Output data should be null on failure");
    }
}

// Share Extraction Error Cases
#[test]
fn test_get_share_null_handle() {
    unsafe {
        let mut share_handle: *mut ShareHandle = ptr::null_mut();
        let result = nomos_da_encoded_data_get_share(
            ptr::null_mut(),
            0,
            &mut share_handle,
        );
        assert_eq!(result, NomosDaResult::ErrorInvalidInput, "Should fail with null handle");
        assert!(share_handle.is_null(), "Share handle should be null on failure");
    }
}

#[test]
fn test_get_share_null_output_handle() {
    unsafe {
        let encoder = nomos_da_encoder_new(4);
        let data = create_test_data(CHUNK_SIZE);
        let mut out_handle: *mut EncodedDataHandle = ptr::null_mut();
        let result = nomos_da_encoder_encode(encoder, data.as_ptr(), data.len(), &mut out_handle);
        assert_eq!(result, NomosDaResult::Success);
        
        let result = nomos_da_encoded_data_get_share(
            out_handle,
            0,
            ptr::null_mut(),
        );
        assert_eq!(result, NomosDaResult::ErrorInvalidInput, "Should fail with null output handle");
        
        nomos_da_encoded_data_free(out_handle);
        nomos_da_encoder_free(encoder);
    }
}
