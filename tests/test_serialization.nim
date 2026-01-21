import unittest
import strutils
import ../src/kzg_nomos_da
import ../src/kzg_nomos_da/types
# BincodeError is re-exported from kzg_nomos_da

proc createTestData(size: int): seq[byte] =
  result = newSeq[byte](size)
  for i in 0 ..< size:
    result[i] = byte((i + 1) mod 256)

suite "nomos-da Serialization Tests":
  setup:
    discard nomos_da_init()
  teardown:
    nomos_da_cleanup()

  test "serializeData and deserializeData roundtrip":
    let originalData = createTestData(100)
    let serialized = serializeData(originalData)
    check serialized.len > 0
    let deserialized = deserializeData(serialized)
    check deserialized == originalData

  test "serializeData with empty data":
    let emptyData: seq[byte] = @[]
    let serialized = serializeData(emptyData)
    check serialized.len > 0
    let deserialized = deserializeData(serialized)
    check deserialized == emptyData

  test "serializeData with single byte":
    let singleByte = @[byte(42)]
    let serialized = serializeData(singleByte)
    let deserialized = deserializeData(serialized)
    check deserialized == singleByte

  test "serializeData with large data":
    let largeData = createTestData(10000)
    let serialized = serializeData(largeData)
    let deserialized = deserializeData(serialized)
    check deserialized == largeData

  test "serializeString and deserializeString roundtrip":
    let originalText = "Hello, world!"
    let serialized = serializeString(originalText)
    check serialized.len > 0
    let deserialized = deserializeString(serialized)
    check deserialized == originalText

  test "serializeString with empty string":
    let emptyString = ""
    let serialized = serializeString(emptyString)
    let deserialized = deserializeString(serialized)
    check deserialized == emptyString

  test "serializeString with unicode characters":
    let unicodeText = "Hello, 世界! 🌍"
    let serialized = serializeString(unicodeText)
    let deserialized = deserializeString(serialized)
    check deserialized == unicodeText

  test "serializeString with long string":
    let longString = "A" & "B".repeat(1000)
    let serialized = serializeString(longString)
    let deserialized = deserializeString(serialized)
    check deserialized == longString

  test "serializeUint16 and deserializeUint16 roundtrip":
    for value in [0'u16, 1'u16, 255'u16, 256'u16, 65535'u16]:
      let serialized = serializeUint16(value)
      check serialized.len > 0
      let deserialized = deserializeUint16(serialized)
      check deserialized == value

  test "deserializeUint16 fails with insufficient data":
    let insufficientData = @[byte(1)]
    expect BincodeError:
      discard deserializeUint16(insufficientData)

  test "deserializeUint16 fails with empty data":
    let emptyData: seq[byte] = @[]
    expect BincodeError:
      discard deserializeUint16(emptyData)

  test "shareToBytes and bytesToShare roundtrip":
    let encoder = newEncoder(columnCount = 4)
    defer:
      freeEncoder(encoder)

    let data = createTestData(CHUNK_SIZE)
    let encoded = encode(encoder, data)
    defer:
      freeEncodedData(encoded)

    let share = getShare(encoded, index = 0)
    defer:
      freeShare(share)

    let originalIndex = getShareIndex(share)
    let serialized = shareToBytes(share)
    check serialized.len > 0

    let deserialized = bytesToShare(serialized)
    check deserialized.index == uint16(originalIndex)

  test "shareToBytes with multiple shares":
    let encoder = newEncoder(columnCount = 4)
    defer:
      freeEncoder(encoder)

    let data = createTestData(CHUNK_SIZE * 2)
    let encoded = encode(encoder, data)
    defer:
      freeEncodedData(encoded)

    let shareCount = getShareCount(encoded)
    for i in 0 ..< shareCount:
      let share = getShare(encoded, index = i)
      defer:
        freeShare(share)

      let originalIndex = getShareIndex(share)
      let serialized = shareToBytes(share)
      let deserialized = bytesToShare(serialized)
      check deserialized.index == uint16(originalIndex)
      check deserialized.index == uint16(i)

  test "shareToBytes fails with null handle":
    let nullShare = ShareHandle(nil)
    expect ValueError:
      discard shareToBytes(nullShare)

  test "encodedDataToBytes and bytesToEncodedData roundtrip":
    let encoder = newEncoder(columnCount = 4)
    defer:
      freeEncoder(encoder)

    let originalData = createTestData(CHUNK_SIZE * 2)
    let encoded = encode(encoder, originalData)
    defer:
      freeEncodedData(encoded)

    let originalShareCount = getShareCount(encoded)
    let serialized = encodedDataToBytes(encoded)
    check serialized.len > 0

    let deserialized = bytesToEncodedData(serialized)
    check deserialized.data == originalData
    check deserialized.shareCount == uint32(originalShareCount)

  test "encodedDataToBytes with various data sizes":
    let encoder = newEncoder(columnCount = 4)
    defer:
      freeEncoder(encoder)

    for size in [CHUNK_SIZE, CHUNK_SIZE * 2, CHUNK_SIZE * 5]:
      let originalData = createTestData(size)
      let encoded = encode(encoder, originalData)
      defer:
        freeEncodedData(encoded)

      let originalShareCount = getShareCount(encoded)
      let serialized = encodedDataToBytes(encoded)
      let deserialized = bytesToEncodedData(serialized)

      check deserialized.data == originalData
      check deserialized.shareCount == uint32(originalShareCount)

  test "encodedDataToBytes with different column counts":
    for columnCount in [2, 4, 8]:
      let encoder = newEncoder(columnCount = columnCount)
      defer:
        freeEncoder(encoder)

      let originalData = createTestData(CHUNK_SIZE)
      let encoded = encode(encoder, originalData)
      defer:
        freeEncodedData(encoded)

      let originalShareCount = getShareCount(encoded)
      let serialized = encodedDataToBytes(encoded)
      let deserialized = bytesToEncodedData(serialized)

      check deserialized.data == originalData
      check deserialized.shareCount == uint32(originalShareCount)

  test "encodedDataToBytes fails with null handle":
    let nullEncoded = EncodedDataHandle(nil)
    expect ValueError:
      discard encodedDataToBytes(nullEncoded)

  test "bytesToEncodedData fails with insufficient data":
    let insufficientData = @[byte(1), 2, 3]
    expect BincodeError:
      discard bytesToEncodedData(insufficientData)

  test "bytesToEncodedData fails with empty data":
    let emptyData: seq[byte] = @[]
    expect BincodeError:
      discard bytesToEncodedData(emptyData)

  test "complete workflow: encode -> serialize -> deserialize -> verify":
    let encoder = newEncoder(columnCount = 4)
    defer:
      freeEncoder(encoder)

    let originalData = createTestData(CHUNK_SIZE * 3)
    let encoded = encode(encoder, originalData)
    defer:
      freeEncodedData(encoded)

    # Serialize encoded data
    let serialized = encodedDataToBytes(encoded)
    check serialized.len > 0

    # Deserialize
    let deserialized = bytesToEncodedData(serialized)
    check deserialized.data == originalData
    check deserialized.shareCount == uint32(getShareCount(encoded))

  test "serialize multiple shares and verify indices":
    let encoder = newEncoder(columnCount = 4)
    defer:
      freeEncoder(encoder)

    let data = createTestData(CHUNK_SIZE * 2)
    let encoded = encode(encoder, data)
    defer:
      freeEncodedData(encoded)

    let shareCount = getShareCount(encoded)
    var serializedShares = newSeq[seq[byte]](shareCount)

    # Serialize all shares
    for i in 0 ..< shareCount:
      let share = getShare(encoded, index = i)
      defer:
        freeShare(share)
      serializedShares[i] = shareToBytes(share)

    # Deserialize and verify
    for i in 0 ..< shareCount:
      let deserialized = bytesToShare(serializedShares[i])
      check deserialized.index == uint16(i)

  test "serializeData preserves data integrity":
    let testCases = @[
      @[byte(0)],
      @[byte(255)],
      @[byte(0), 255],
      @[byte(255), 0],
      createTestData(100),
      createTestData(1000),
    ]

    for originalData in testCases:
      let serialized = serializeData(originalData)
      let deserialized = deserializeData(serialized)
      check deserialized == originalData

  test "serializeString preserves string integrity":
    let testCases = @[
      "",
      "a",
      "Hello",
      "Hello, world!",
      "Test with\nnewlines",
      "Test with\ttabs",
      "Test with \"quotes\"",
      "Test with 'single quotes'",
      "Unicode: 世界 🌍",
    ]

    for originalText in testCases:
      let serialized = serializeString(originalText)
      let deserialized = deserializeString(serialized)
      check deserialized == originalText

  test "serializeUint16 edge cases":
    let edgeCases = @[0'u16, 1'u16, 255'u16, 256'u16, 65534'u16, 65535'u16]

    for value in edgeCases:
      let serialized = serializeUint16(value)
      let deserialized = deserializeUint16(serialized)
      check deserialized == value
