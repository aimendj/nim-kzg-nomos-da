//! Integration tests for nomos-da FFI wrapper

use logos_blockchain_kzgrs_backend::encoder::DaEncoderParams;
use nomos_da_ffi::{
    nomos_da_cleanup, nomos_da_commitments_free,
    nomos_da_encoder_encode, nomos_da_encoder_free,
    nomos_da_encoder_new, nomos_da_encoded_data_free,
    nomos_da_encoded_data_get_data, nomos_da_encoded_data_get_share,
    nomos_da_encoded_data_get_share_count,
    nomos_da_init, nomos_da_reconstruct, nomos_da_reconstruct_free,
    nomos_da_share_free,
    nomos_da_share_get_commitments, nomos_da_share_get_index, nomos_da_verifier_free,
    nomos_da_verifier_new, nomos_da_verifier_verify, CommitmentsHandle, EncodedDataHandle,
    NomosDaResult, ShareHandle,
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
fn test_encode_not_multiple_of_chunk_size() {
    unsafe {
        let column_count = 4;
        let encoder = nomos_da_encoder_new(column_count);
        assert!(!encoder.is_null(), "Encoder should be created (column_count: {}, chunk_size: {})", column_count, CHUNK_SIZE);

        // Test with data length that is not a multiple of chunk size
        let invalid_sizes = [1, CHUNK_SIZE - 1, CHUNK_SIZE + 1, 2 * CHUNK_SIZE - 1];
        
        for data_size in invalid_sizes.iter() {
            let data = create_test_data(*data_size);
            let mut out_handle: *mut EncodedDataHandle = ptr::null_mut();
            let result = nomos_da_encoder_encode(
                encoder,
                data.as_ptr(),
                data.len(),
                &mut out_handle,
            );
            assert_eq!(result, NomosDaResult::ErrorInvalidInput, "Encoding should fail when data length is not a multiple of chunk size (data_size: {}, chunk_size: {})", data_size, CHUNK_SIZE);
            assert!(out_handle.is_null(), "Output handle should be null on failure (data_size: {}, chunk_size: {})", data_size, CHUNK_SIZE);
        }

        nomos_da_encoder_free(encoder);
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

#[test]
fn test_share_get_index() {
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

        let share_count = nomos_da_encoded_data_get_share_count(out_handle);
        assert_eq!(share_count, column_count, "Share count should equal column_count (share_count: {}, column_count: {}, chunk_size: {})", share_count, column_count, CHUNK_SIZE);

        // Test getting index from multiple shares
        for i in 0..share_count {
            let mut share_handle: *mut ShareHandle = ptr::null_mut();
            let result = nomos_da_encoded_data_get_share(out_handle, i, &mut share_handle);
            assert_eq!(result, NomosDaResult::Success, "Should successfully get share (share_index: {}, column_count: {}, chunk_size: {})", i, column_count, CHUNK_SIZE);
            assert!(!share_handle.is_null(), "Share handle should not be null (share_index: {}, chunk_size: {})", i, CHUNK_SIZE);

            let share_idx = nomos_da_share_get_index(share_handle);
            assert_eq!(share_idx, i as u16, "Share index should match (expected: {}, got: {}, chunk_size: {})", i, share_idx, CHUNK_SIZE);

            nomos_da_share_free(share_handle);
        }

        // Test null handle returns 0
        let null_idx = nomos_da_share_get_index(ptr::null_mut());
        assert_eq!(null_idx, 0, "Null share handle should return index 0 (chunk_size: {})", CHUNK_SIZE);

        nomos_da_encoded_data_free(out_handle);
        nomos_da_encoder_free(encoder);
    }
}

#[test]
fn test_share_get_commitments_null_handles() {
    unsafe {
        let mut commitments_handle: *mut CommitmentsHandle = ptr::null_mut();

        let result_null_share = nomos_da_share_get_commitments(ptr::null_mut(), &mut commitments_handle);
        assert_eq!(result_null_share, NomosDaResult::ErrorInvalidInput, "Should fail with null share handle");

        let result_null_output = nomos_da_share_get_commitments(ptr::null_mut(), ptr::null_mut());
        assert_eq!(result_null_output, NomosDaResult::ErrorInvalidInput, "Should fail with null output handle");
    }
}

// ============================================================================
// Data Reconstruction Tests
// ============================================================================

#[test]
fn test_reconstruct_from_all_shares() {
    unsafe {
        let column_count = 4;
        let encoder = nomos_da_encoder_new(column_count);
        assert!(!encoder.is_null(), "Encoder should be created (column_count: {}, chunk_size: {})", column_count, CHUNK_SIZE);

        let original_data = create_test_data(CHUNK_SIZE);
        let mut out_handle: *mut EncodedDataHandle = ptr::null_mut();
        let result = nomos_da_encoder_encode(
            encoder,
            original_data.as_ptr(),
            original_data.len(),
            &mut out_handle,
        );
        assert_eq!(result, NomosDaResult::Success, "Encoding should succeed (column_count: {}, chunk_size: {})", column_count, CHUNK_SIZE);

        let share_count = nomos_da_encoded_data_get_share_count(out_handle);
        assert_eq!(share_count, column_count, "Share count should equal column_count (share_count: {}, column_count: {}, chunk_size: {})", share_count, column_count, CHUNK_SIZE);

        // For reconstruction, we only need the first column_count/2 shares (original columns)
        // The remaining shares are RS-encoded redundancy
        let original_share_count = column_count / 2;

        let mut share_handles: Vec<*mut ShareHandle> = Vec::with_capacity(original_share_count);
        for i in 0..original_share_count {
            let mut share_handle: *mut ShareHandle = ptr::null_mut();
            let result = nomos_da_encoded_data_get_share(out_handle, i, &mut share_handle);
            assert_eq!(result, NomosDaResult::Success, "Should successfully get share (share_index: {}, column_count: {}, chunk_size: {})", i, column_count, CHUNK_SIZE);
            assert!(!share_handle.is_null(), "Share handle should not be null (share_index: {}, chunk_size: {})", i, CHUNK_SIZE);
            share_handles.push(share_handle);
        }

        let mut reconstructed_data: *mut u8 = ptr::null_mut();
        let mut reconstructed_len: usize = 0;
        let reconstruct_result = nomos_da_reconstruct(
            share_handles.as_ptr(),
            share_handles.len(),
            &mut reconstructed_data,
            &mut reconstructed_len,
        );
        assert_eq!(reconstruct_result, NomosDaResult::Success, "Reconstruction should succeed (share_count: {}, chunk_size: {})", share_count, CHUNK_SIZE);
        assert!(!reconstructed_data.is_null(), "Reconstructed data should not be null (chunk_size: {})", CHUNK_SIZE);
        assert!(reconstructed_len > 0, "Reconstructed length should be greater than 0 (reconstructed_len: {}, chunk_size: {})", reconstructed_len, CHUNK_SIZE);

        // Calculate expected reconstructed size (padded to row boundaries)
        // Row size = (column_count / 2) * chunk_size
        let row_size = (column_count / 2) * CHUNK_SIZE;
        let num_rows = (original_data.len() + row_size - 1) / row_size; // Ceiling division
        let expected_reconstructed_len = num_rows * row_size;
        
        assert_eq!(reconstructed_len, expected_reconstructed_len, "Reconstructed length should match expected padded size (reconstructed_len: {}, expected_len: {}, original_len: {}, row_size: {}, num_rows: {}, chunk_size: {})", reconstructed_len, expected_reconstructed_len, original_data.len(), row_size, num_rows, CHUNK_SIZE);
        
        let reconstructed_slice = std::slice::from_raw_parts(reconstructed_data, reconstructed_len);
        // Check that the original data matches the beginning of reconstructed data
        assert_eq!(&reconstructed_slice[..original_data.len()], original_data.as_slice(), "Reconstructed data prefix should match original (reconstructed_len: {}, original_len: {}, chunk_size: {})", reconstructed_len, original_data.len(), CHUNK_SIZE);
        // Check that the padding is zeros
        if reconstructed_len > original_data.len() {
            let padding = &reconstructed_slice[original_data.len()..];
            assert!(padding.iter().all(|&b| b == 0), "Padding should be zeros (reconstructed_len: {}, original_len: {}, chunk_size: {})", reconstructed_len, original_data.len(), CHUNK_SIZE);
        }

        for share_handle in share_handles {
            nomos_da_share_free(share_handle);
        }
        nomos_da_reconstruct_free(reconstructed_data, reconstructed_len);
        nomos_da_encoded_data_free(out_handle);
        nomos_da_encoder_free(encoder);
    }
}

#[test]
fn test_reconstruct_different_data_sizes() {
    unsafe {
        let column_count = 4;
        let encoder = nomos_da_encoder_new(column_count);
        assert!(!encoder.is_null(), "Encoder should be created (column_count: {}, chunk_size: {})", column_count, CHUNK_SIZE);

        for data_size in [3 * CHUNK_SIZE, 2 * CHUNK_SIZE, CHUNK_SIZE] {
            let original_data = create_test_data(data_size);
            let mut out_handle: *mut EncodedDataHandle = ptr::null_mut();
            let result = nomos_da_encoder_encode(
                encoder,
                original_data.as_ptr(),
                original_data.len(),
                &mut out_handle,
            );
            assert_eq!(result, NomosDaResult::Success, "Encoding should succeed (data_size: {}, column_count: {}, chunk_size: {})", data_size, column_count, CHUNK_SIZE);

            let share_count = nomos_da_encoded_data_get_share_count(out_handle);
            let original_share_count = column_count / 2;
            let mut share_handles: Vec<*mut ShareHandle> = Vec::with_capacity(original_share_count);
            for i in 0..original_share_count {
                let mut share_handle: *mut ShareHandle = ptr::null_mut();
                let result = nomos_da_encoded_data_get_share(out_handle, i, &mut share_handle);
                assert_eq!(result, NomosDaResult::Success, "Should successfully get share (share_index: {}, data_size: {}, chunk_size: {})", i, data_size, CHUNK_SIZE);
                share_handles.push(share_handle);
            }

            let mut reconstructed_data: *mut u8 = ptr::null_mut();
            let mut reconstructed_len: usize = 0;
            let reconstruct_result = nomos_da_reconstruct(
                share_handles.as_ptr(),
                share_handles.len(),
                &mut reconstructed_data,
                &mut reconstructed_len,
            );
            assert_eq!(reconstruct_result, NomosDaResult::Success, "Reconstruction should succeed (data_size: {}, share_count: {}, chunk_size: {})", data_size, share_count, CHUNK_SIZE);

            // Calculate expected reconstructed size (padded to row boundaries)
            // Row size = (column_count / 2) * chunk_size
            let row_size = (column_count / 2) * CHUNK_SIZE;
            let num_rows = (original_data.len() + row_size - 1) / row_size; // Ceiling division
            let expected_reconstructed_len = num_rows * row_size;
            
            assert_eq!(reconstructed_len, expected_reconstructed_len, "Reconstructed length should match expected padded size (data_size: {}, reconstructed_len: {}, expected_len: {}, row_size: {}, num_rows: {}, chunk_size: {})", data_size, reconstructed_len, expected_reconstructed_len, row_size, num_rows, CHUNK_SIZE);
            
            let reconstructed_slice = std::slice::from_raw_parts(reconstructed_data, reconstructed_len);
            // Check that the original data matches the beginning of reconstructed data
            assert_eq!(&reconstructed_slice[..original_data.len()], original_data.as_slice(), "Reconstructed data prefix should match original (data_size: {}, reconstructed_len: {}, original_len: {}, chunk_size: {})", data_size, reconstructed_len, original_data.len(), CHUNK_SIZE);
            // Check that the padding is zeros
            if reconstructed_len > original_data.len() {
                let padding = &reconstructed_slice[original_data.len()..];
                assert!(padding.iter().all(|&b| b == 0), "Padding should be zeros (data_size: {}, reconstructed_len: {}, original_len: {}, chunk_size: {})", data_size, reconstructed_len, original_data.len(), CHUNK_SIZE);
            }

            for share_handle in share_handles {
                nomos_da_share_free(share_handle);
            }
            nomos_da_reconstruct_free(reconstructed_data, reconstructed_len);
            nomos_da_encoded_data_free(out_handle);
        }

        nomos_da_encoder_free(encoder);
    }
}

#[test]
fn test_reconstruct_null_handles() {
    unsafe {
        let mut data: *mut u8 = ptr::null_mut();
        let mut len: usize = 0;

        let result_null_shares = nomos_da_reconstruct(ptr::null(), 4, &mut data, &mut len);
        assert_eq!(result_null_shares, NomosDaResult::ErrorInvalidInput, "Reconstruction should fail with null shares array");

        let result_null_output = nomos_da_reconstruct(ptr::null(), 4, ptr::null_mut(), &mut len);
        assert_eq!(result_null_output, NomosDaResult::ErrorInvalidInput, "Reconstruction should fail with null output data pointer");

        let result_null_len = nomos_da_reconstruct(ptr::null(), 4, &mut data, ptr::null_mut());
        assert_eq!(result_null_len, NomosDaResult::ErrorInvalidInput, "Reconstruction should fail with null length pointer");

        let result_zero_count = nomos_da_reconstruct(ptr::null(), 0, &mut data, &mut len);
        assert_eq!(result_zero_count, NomosDaResult::ErrorInvalidInput, "Reconstruction should fail with zero share count");
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
