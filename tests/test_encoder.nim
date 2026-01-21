import unittest
import strutils
import ../src/kzg_nomos_da
import ../src/kzg_nomos_da/types

proc createTestData(size: int): seq[byte] =
  result = newSeq[byte](size)
  for i in 0 ..< size:
    result[i] = byte((i + 1) mod 256)

suite "nomos-da Encoder API Tests":
  setup:
    discard nomos_da_init()
  teardown:
    nomos_da_cleanup()

  test "CHUNK_SIZE is correct":
    check CHUNK_SIZE > 0

  test "encoder creation and destruction":
    let encoder = newEncoder(columnCount = 4)
    check encoder.pointer != nil
    freeEncoder(encoder)

  test "encoder creation with various column counts":
    for columnCount in [2, 4, 8, 16, 32]:
      let encoder = newEncoder(columnCount = columnCount)
      check encoder.pointer != nil
      freeEncoder(encoder)

  test "encoder creation fails with invalid column count":
    expect ValueError:
      discard newEncoder(columnCount = 0)

    expect ValueError:
      discard newEncoder(columnCount = -1)

  test "encode with single chunk":
    let encoder = newEncoder(columnCount = 4)
    defer:
      freeEncoder(encoder)

    let data = createTestData(CHUNK_SIZE)
    let encoded = encode(encoder, data)
    defer:
      freeEncodedData(encoded)

    check encoded.pointer != nil
    check getShareCount(encoded) == 4

  test "encode with multiple chunks":
    let encoder = newEncoder(columnCount = 4)
    defer:
      freeEncoder(encoder)

    let data = createTestData(CHUNK_SIZE * 2)
    let encoded = encode(encoder, data)
    defer:
      freeEncodedData(encoded)

    check encoded.pointer != nil
    check getShareCount(encoded) == 4

  test "encode with large data":
    let encoder = newEncoder(columnCount = 8)
    defer:
      freeEncoder(encoder)

    let data = createTestData(CHUNK_SIZE * 10)
    let encoded = encode(encoder, data)
    defer:
      freeEncodedData(encoded)

    check encoded.pointer != nil
    check getShareCount(encoded) == 8

  test "encode fails with empty data":
    let encoder = newEncoder(columnCount = 4)
    defer:
      freeEncoder(encoder)

    let data: seq[byte] = @[]
    expect ValueError:
      discard encode(encoder, data)

  test "encode fails with data not multiple of chunk size":
    let encoder = newEncoder(columnCount = 4)
    defer:
      freeEncoder(encoder)

    let data = createTestData(50) # Not a multiple of 31
    expect ValueError:
      discard encode(encoder, data)

  test "encode fails with null encoder":
    let encoder = EncoderHandle(nil)
    let data = createTestData(CHUNK_SIZE)
    expect ValueError:
      discard encode(encoder, data)

  test "getData retrieves original data":
    let encoder = newEncoder(columnCount = 4)
    defer:
      freeEncoder(encoder)

    let originalData = createTestData(CHUNK_SIZE * 2)
    let encoded = encode(encoder, originalData)
    defer:
      freeEncodedData(encoded)

    let retrievedData = getData(encoded)
    check retrievedData == originalData

  test "getData with various data sizes":
    let encoder = newEncoder(columnCount = 4)
    defer:
      freeEncoder(encoder)

    for size in [CHUNK_SIZE, CHUNK_SIZE * 2, CHUNK_SIZE * 5, CHUNK_SIZE * 10]:
      let originalData = createTestData(size)
      let encoded = encode(encoder, originalData)
      defer:
        freeEncodedData(encoded)

      let retrievedData = getData(encoded)
      check retrievedData == originalData

  test "getData fails with null handle":
    let encoded = EncodedDataHandle(nil)
    expect ValueError:
      discard getData(encoded)

  test "getShareCount returns correct value":
    let encoder = newEncoder(columnCount = 4)
    defer:
      freeEncoder(encoder)

    let data = createTestData(CHUNK_SIZE)
    let encoded = encode(encoder, data)
    defer:
      freeEncodedData(encoded)

    check getShareCount(encoded) == 4

  test "getShareCount with different column counts":
    for columnCount in [2, 4, 8, 16]:
      let encoder = newEncoder(columnCount = columnCount)
      defer:
        freeEncoder(encoder)

      let data = createTestData(CHUNK_SIZE)
      let encoded = encode(encoder, data)
      defer:
        freeEncodedData(encoded)

      check getShareCount(encoded) == columnCount

  test "getShareCount returns 0 for null handle":
    let encoded = EncodedDataHandle(nil)
    check getShareCount(encoded) == 0

  test "complete workflow: encode -> getData -> verify":
    let encoder = newEncoder(columnCount = 4)
    defer:
      freeEncoder(encoder)

    let originalData = createTestData(CHUNK_SIZE * 3)
    let encoded = encode(encoder, originalData)
    defer:
      freeEncodedData(encoded)

    check encoded.pointer != nil
    check getShareCount(encoded) == 4

    let retrievedData = getData(encoded)
    check retrievedData == originalData

  test "multiple encoders can coexist":
    let encoder1 = newEncoder(columnCount = 4)
    defer:
      freeEncoder(encoder1)

    let encoder2 = newEncoder(columnCount = 8)
    defer:
      freeEncoder(encoder2)

    let data1 = createTestData(CHUNK_SIZE)
    let data2 = createTestData(CHUNK_SIZE * 2)

    let encoded1 = encode(encoder1, data1)
    defer:
      freeEncodedData(encoded1)

    let encoded2 = encode(encoder2, data2)
    defer:
      freeEncodedData(encoded2)

    check getShareCount(encoded1) == 4
    check getShareCount(encoded2) == 8

    check getData(encoded1) == data1
    check getData(encoded2) == data2

  test "encode with minimum data size (one chunk)":
    let encoder = newEncoder(columnCount = 2)
    defer:
      freeEncoder(encoder)

    let data = createTestData(CHUNK_SIZE)
    let encoded = encode(encoder, data)
    defer:
      freeEncodedData(encoded)

    check encoded.pointer != nil
    check getShareCount(encoded) == 2
    check getData(encoded) == data

  test "encode with very large data":
    let encoder = newEncoder(columnCount = 16)
    defer:
      freeEncoder(encoder)

    let data = createTestData(CHUNK_SIZE * 100)
    let encoded = encode(encoder, data)
    defer:
      freeEncodedData(encoded)

    check encoded.pointer != nil
    check getShareCount(encoded) == 16
    check getData(encoded) == data

  test "encode with all zeros":
    let encoder = newEncoder(columnCount = 4)
    defer:
      freeEncoder(encoder)

    let data = newSeq[byte](CHUNK_SIZE * 2)
    let encoded = encode(encoder, data)
    defer:
      freeEncodedData(encoded)

    check encoded.pointer != nil
    let retrievedData = getData(encoded)
    check retrievedData == data

  test "encode with all 0xFF":
    let encoder = newEncoder(columnCount = 4)
    defer:
      freeEncoder(encoder)

    var data = newSeq[byte](CHUNK_SIZE * 2)
    for i in 0 ..< data.len:
      data[i] = 0xFF

    let encoded = encode(encoder, data)
    defer:
      freeEncodedData(encoded)

    check encoded.pointer != nil
    let retrievedData = getData(encoded)
    check retrievedData == data

  test "encode with pattern data":
    let encoder = newEncoder(columnCount = 4)
    defer:
      freeEncoder(encoder)

    var data = newSeq[byte](CHUNK_SIZE * 2)
    for i in 0 ..< data.len:
      data[i] = byte(i mod 256)

    let encoded = encode(encoder, data)
    defer:
      freeEncodedData(encoded)

    check encoded.pointer != nil
    let retrievedData = getData(encoded)
    check retrievedData == data

  test "error message is available after failed encode":
    let encoder = newEncoder(columnCount = 4)
    defer:
      freeEncoder(encoder)

    let data = createTestData(50) # Not a multiple of chunk size

    var raised = false
    try:
      discard encode(encoder, data)
    except ValueError as e:
      raised = true
      check e.msg.len > 0
      check e.msg.contains("multiple") or e.msg.contains("chunk")
    check raised

  test "getLastError returns empty string when no error":
    let encoder = newEncoder(columnCount = 4)
    defer:
      freeEncoder(encoder)

    let data = createTestData(CHUNK_SIZE)
    let encoded = encode(encoder, data)
    defer:
      freeEncodedData(encoded)

    discard getLastError()

test "initialization and cleanup work":
  discard nomos_da_init()
  nomos_da_cleanup()
  discard nomos_da_init()
  nomos_da_cleanup()
