import unittest
import ../src/kzg_nomos_da
import ../src/kzg_nomos_da/types

proc createTestData(size: int): seq[byte] =
  result = newSeq[byte](size)
  for i in 0 ..< size:
    result[i] = byte((i + 1) mod 256)

suite "nomos-da Share API Tests":
  setup:
    discard nomos_da_init()
  teardown:
    nomos_da_cleanup()

  test "getShare from encoded data":
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

    check share.pointer != nil

  test "getShare from all shares":
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

      check share.pointer != nil
      check getShareIndex(share) == i

  test "getShare fails with invalid index":
    let encoder = newEncoder(columnCount = 4)
    defer:
      freeEncoder(encoder)

    let data = createTestData(CHUNK_SIZE)
    let encoded = encode(encoder, data)
    defer:
      freeEncodedData(encoded)

    let shareCount = getShareCount(encoded)
    expect ValueError:
      discard getShare(encoded, index = shareCount)

    expect ValueError:
      discard getShare(encoded, index = -1)

  test "getShare fails with null encoded data":
    let encoded = EncodedDataHandle(nil)
    expect ValueError:
      discard getShare(encoded, index = 0)

  test "getShareIndex returns correct index":
    let encoder = newEncoder(columnCount = 4)
    defer:
      freeEncoder(encoder)

    let data = createTestData(CHUNK_SIZE)
    let encoded = encode(encoder, data)
    defer:
      freeEncodedData(encoded)

    for i in 0 ..< 4:
      let share = getShare(encoded, index = i)
      defer:
        freeShare(share)

      check getShareIndex(share) == i

  test "getShareIndex returns 0 for null handle":
    let share = ShareHandle(nil)
    check getShareIndex(share) == 0

  test "freeShare is safe with null handle":
    let share = ShareHandle(nil)
    freeShare(share)

  test "getShare with different column counts":
    for columnCount in [2, 4, 8, 16]:
      let encoder = newEncoder(columnCount = columnCount)
      defer:
        freeEncoder(encoder)

      let data = createTestData(CHUNK_SIZE)
      let encoded = encode(encoder, data)
      defer:
        freeEncodedData(encoded)

      for i in 0 ..< columnCount:
        let share = getShare(encoded, index = i)
        defer:
          freeShare(share)

        check share.pointer != nil
        check getShareIndex(share) == i

  test "getShare with various data sizes":
    let encoder = newEncoder(columnCount = 4)
    defer:
      freeEncoder(encoder)

    for dataSize in [CHUNK_SIZE, CHUNK_SIZE * 2, CHUNK_SIZE * 4]:
      let data = createTestData(dataSize)
      let encoded = encode(encoder, data)
      defer:
        freeEncodedData(encoded)

      let share = getShare(encoded, index = 0)
      defer:
        freeShare(share)

      check share.pointer != nil
      check getShareIndex(share) == 0

  test "multiple shares can coexist":
    let encoder = newEncoder(columnCount = 4)
    defer:
      freeEncoder(encoder)

    let data = createTestData(CHUNK_SIZE)
    let encoded = encode(encoder, data)
    defer:
      freeEncodedData(encoded)

    let share1 = getShare(encoded, index = 0)
    defer:
      freeShare(share1)

    let share2 = getShare(encoded, index = 1)
    defer:
      freeShare(share2)

    let share3 = getShare(encoded, index = 2)
    defer:
      freeShare(share3)

    check share1.pointer != nil
    check share2.pointer != nil
    check share3.pointer != nil

    check getShareIndex(share1) == 0
    check getShareIndex(share2) == 1
    check getShareIndex(share3) == 2

  test "getCommitments from share":
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

    let commitments = getCommitments(share)
    defer:
      freeCommitments(commitments)

    check commitments.pointer != nil

  test "getCommitments from all shares":
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

      let commitments = getCommitments(share)
      defer:
        freeCommitments(commitments)

      check commitments.pointer != nil
      check getShareIndex(share) == i

  test "getCommitments fails with null share":
    let share = ShareHandle(nil)
    expect ValueError:
      discard getCommitments(share)

  test "getCommitments with different column counts":
    for columnCount in [2, 4, 8, 16]:
      let encoder = newEncoder(columnCount = columnCount)
      defer:
        freeEncoder(encoder)

      let data = createTestData(CHUNK_SIZE)
      let encoded = encode(encoder, data)
      defer:
        freeEncodedData(encoded)

      let share = getShare(encoded, index = 0)
      defer:
        freeShare(share)

      let commitments = getCommitments(share)
      defer:
        freeCommitments(commitments)

      check commitments.pointer != nil

  test "getCommitments with various data sizes":
    let encoder = newEncoder(columnCount = 4)
    defer:
      freeEncoder(encoder)

    for dataSize in [CHUNK_SIZE, CHUNK_SIZE * 2, CHUNK_SIZE * 4]:
      let data = createTestData(dataSize)
      let encoded = encode(encoder, data)
      defer:
        freeEncodedData(encoded)

      let share = getShare(encoded, index = 0)
      defer:
        freeShare(share)

      let commitments = getCommitments(share)
      defer:
        freeCommitments(commitments)

      check commitments.pointer != nil

  test "freeCommitments is safe with null handle":
    let commitments = CommitmentsHandle(nil)
    freeCommitments(commitments)

  test "multiple commitments can coexist":
    let encoder = newEncoder(columnCount = 4)
    defer:
      freeEncoder(encoder)

    let data = createTestData(CHUNK_SIZE)
    let encoded = encode(encoder, data)
    defer:
      freeEncodedData(encoded)

    let share1 = getShare(encoded, index = 0)
    defer:
      freeShare(share1)

    let share2 = getShare(encoded, index = 1)
    defer:
      freeShare(share2)

    let commitments1 = getCommitments(share1)
    defer:
      freeCommitments(commitments1)

    let commitments2 = getCommitments(share2)
    defer:
      freeCommitments(commitments2)

    check commitments1.pointer != nil
    check commitments2.pointer != nil
