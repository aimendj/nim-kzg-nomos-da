import unittest
import ../src/kzg_nomos_da
import ../src/kzg_nomos_da/types

proc createTestData(size: int): seq[byte] =
  result = newSeq[byte](size)
  for i in 0..<size:
    result[i] = byte((i + 1) mod 256)

suite "nomos-da Verifier API Tests":
  setup:
    discard nomos_da_init()
  teardown:
    nomos_da_cleanup()

  test "verifier creation and destruction":
    let verifier = newVerifier()
    check verifier.pointer != nil
    freeVerifier(verifier)

  test "verify share":
    let columnCount = 4
    let encoder = newEncoder(columnCount = columnCount)
    defer: freeEncoder(encoder)
    
    let data = createTestData(CHUNK_SIZE)
    let encoded = encode(encoder, data)
    defer: freeEncodedData(encoded)
    
    let share = getShare(encoded, index = 0)
    defer: freeShare(share)
    
    let verifier = newVerifier()
    defer: freeVerifier(verifier)
    
    let isValid = verify(verifier, share, rowsDomainSize = columnCount)
    check isValid

  test "verify all shares from encoded data":
    let columnCount = 4
    let encoder = newEncoder(columnCount = columnCount)
    defer: freeEncoder(encoder)
    
    let data = createTestData(CHUNK_SIZE * 2)
    let encoded = encode(encoder, data)
    defer: freeEncodedData(encoded)
    
    let verifier = newVerifier()
    defer: freeVerifier(verifier)
    
    let shareCount = getShareCount(encoded)
    for i in 0..<shareCount:
      let share = getShare(encoded, index = i)
      defer: freeShare(share)
      
      let isValid = verify(verifier, share, rowsDomainSize = columnCount)
      check isValid
      check getShareIndex(share) == i

  test "verify with different column counts":
    for columnCount in [2, 4, 8, 16]:
      let encoder = newEncoder(columnCount = columnCount)
      defer: freeEncoder(encoder)
      
      let data = createTestData(CHUNK_SIZE)
      let encoded = encode(encoder, data)
      defer: freeEncodedData(encoded)
      
      let share = getShare(encoded, index = 0)
      defer: freeShare(share)
      
      let verifier = newVerifier()
      defer: freeVerifier(verifier)
      
      let isValid = verify(verifier, share, rowsDomainSize = columnCount)
      check isValid

  test "verify share with various data sizes":
    let columnCount = 4
    let encoder = newEncoder(columnCount = columnCount)
    defer: freeEncoder(encoder)
    
    for dataSize in [CHUNK_SIZE, CHUNK_SIZE * 2, CHUNK_SIZE * 4]:
      let data = createTestData(dataSize)
      let encoded = encode(encoder, data)
      defer: freeEncodedData(encoded)
      
      let share = getShare(encoded, index = 0)
      defer: freeShare(share)
      
      let verifier = newVerifier()
      defer: freeVerifier(verifier)
      
      let isValid = verify(verifier, share, rowsDomainSize = columnCount)
      check isValid

  test "verify fails with null verifier":
    let encoder = newEncoder(columnCount = 4)
    defer: freeEncoder(encoder)
    
    let data = createTestData(CHUNK_SIZE)
    let encoded = encode(encoder, data)
    defer: freeEncodedData(encoded)
    
    let share = getShare(encoded, index = 0)
    defer: freeShare(share)
    
    let verifier = VerifierHandle(nil)
    expect ValueError:
      discard verify(verifier, share, rowsDomainSize = 4)

  test "verify fails with null share":
    let verifier = newVerifier()
    defer: freeVerifier(verifier)
    
    let share = ShareHandle(nil)
    expect ValueError:
      discard verify(verifier, share, rowsDomainSize = 4)

  test "verify fails with invalid rowDomainSize":
    let encoder = newEncoder(columnCount = 4)
    defer: freeEncoder(encoder)
    
    let data = createTestData(CHUNK_SIZE)
    let encoded = encode(encoder, data)
    defer: freeEncodedData(encoded)
    
    let share = getShare(encoded, index = 0)
    defer: freeShare(share)
    
    let verifier = newVerifier()
    defer: freeVerifier(verifier)
    
    expect ValueError:
      discard verify(verifier, share, rowsDomainSize = 0)
    
    expect ValueError:
      discard verify(verifier, share, rowsDomainSize = -1)

  test "multiple verifiers can coexist":
    let verifier1 = newVerifier()
    defer: freeVerifier(verifier1)
    
    let verifier2 = newVerifier()
    defer: freeVerifier(verifier2)
    
    let encoder = newEncoder(columnCount = 4)
    defer: freeEncoder(encoder)
    
    let data = createTestData(CHUNK_SIZE)
    let encoded = encode(encoder, data)
    defer: freeEncodedData(encoded)
    
    let share = getShare(encoded, index = 0)
    defer: freeShare(share)
    
    let isValid1 = verify(verifier1, share, rowsDomainSize = 4)
    let isValid2 = verify(verifier2, share, rowsDomainSize = 4)
    
    check isValid1
    check isValid2

  test "verify shares from different encoded data":
    let encoder = newEncoder(columnCount = 4)
    defer: freeEncoder(encoder)
    
    let data1 = createTestData(CHUNK_SIZE)
    let data2 = createTestData(CHUNK_SIZE * 2)
    
    let encoded1 = encode(encoder, data1)
    defer: freeEncodedData(encoded1)
    
    let encoded2 = encode(encoder, data2)
    defer: freeEncodedData(encoded2)
    
    let share1 = getShare(encoded1, index = 0)
    defer: freeShare(share1)
    
    let share2 = getShare(encoded2, index = 0)
    defer: freeShare(share2)
    
    let verifier = newVerifier()
    defer: freeVerifier(verifier)
    
    let isValid1 = verify(verifier, share1, rowsDomainSize = 4)
    let isValid2 = verify(verifier, share2, rowsDomainSize = 4)
    
    check isValid1
    check isValid2
    check getShareIndex(share1) == 0
    check getShareIndex(share2) == 0

  test "verify with large data":
    let columnCount = 8
    let encoder = newEncoder(columnCount = columnCount)
    defer: freeEncoder(encoder)
    
    let data = createTestData(CHUNK_SIZE * 10)
    let encoded = encode(encoder, data)
    defer: freeEncodedData(encoded)
    
    let verifier = newVerifier()
    defer: freeVerifier(verifier)
    
    let shareCount = getShareCount(encoded)
    for i in 0..<shareCount:
      let share = getShare(encoded, index = i)
      defer: freeShare(share)
      
      let isValid = verify(verifier, share, rowsDomainSize = columnCount)
      check isValid
