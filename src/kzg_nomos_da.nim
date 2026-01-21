## Nim wrapper for nomos-da Rust library
##
## This module provides Nim bindings for the nomos-da data availability layer
## from the logos-blockchain repository.
##
## To use this wrapper:
## 1. Build the Rust FFI library: `make build-rust` or
##    `cd ffi-wrapper && cargo build --release`
## 2. Link the static library in your Nim code or via nim.cfg

{.push raises: [], gcsafe.}

import kzg_nomos_da/types

const CHUNK_SIZE* = 31

proc nomos_da_init*(): cint {.importc: "nomos_da_init".}
proc nomos_da_cleanup*() {.importc: "nomos_da_cleanup".}
proc nomos_da_free_string*(s: cstring) {.importc: "nomos_da_free_string".}
proc nomos_da_get_last_error*(): cstring {.importc: "nomos_da_get_last_error".}

proc getLastError*(): string =
  let errMsg = nomos_da_get_last_error()
  if errMsg != nil:
    let msg = $errMsg
    nomos_da_free_string(errMsg)
    msg
  else:
    ""

proc checkResult*(
    result: NomosDaResult, operation: string = ""
): void {.raises: [ValueError].} =
  if result != Success:
    let errMsg = getLastError()
    let msg =
      (if operation.len > 0: operation & " failed: " else: "") &
      "nomos-da operation failed with code: " & $result &
      (if errMsg.len > 0: " (" & errMsg & ")" else: "")
    raise newException(ValueError, msg)

proc nomos_da_encoder_new(
  column_count: CSizeT
): pointer {.importc: "nomos_da_encoder_new".}

proc nomos_da_encoder_free(handle: pointer) {.importc: "nomos_da_encoder_free".}
proc nomos_da_encoder_encode(
  encoder: pointer, data: ptr uint8, data_len: CSizeT, out_handle: ptr pointer
): NomosDaResult {.importc: "nomos_da_encoder_encode".}

proc nomos_da_encoded_data_free(
  handle: pointer
) {.importc: "nomos_da_encoded_data_free".}

proc nomos_da_encoded_data_get_data(
  handle: pointer, out_data: ptr uint8, out_len: ptr CSizeT
): NomosDaResult {.importc: "nomos_da_encoded_data_get_data".}

proc nomos_da_encoded_data_get_share_count(
  handle: pointer
): CSizeT {.importc: "nomos_da_encoded_data_get_share_count".}

proc nomos_da_encoded_data_get_share(
  handle: pointer, index: CSizeT, out_share_handle: ptr pointer
): NomosDaResult {.importc: "nomos_da_encoded_data_get_share".}

proc nomos_da_verifier_new(): pointer {.importc: "nomos_da_verifier_new".}
proc nomos_da_verifier_free(handle: pointer) {.importc: "nomos_da_verifier_free".}
proc nomos_da_verifier_verify(
  verifier: pointer, share_handle: pointer, rows_domain_size: CSizeT
): bool {.importc: "nomos_da_verifier_verify".}

proc nomos_da_share_free(handle: pointer) {.importc: "nomos_da_share_free".}
proc nomos_da_share_get_index(
  share_handle: pointer
): uint16 {.importc: "nomos_da_share_get_index".}

proc nomos_da_share_get_commitments(
  share_handle: pointer, out_commitments_handle: ptr pointer
): NomosDaResult {.importc: "nomos_da_share_get_commitments".}

proc nomos_da_commitments_free(handle: pointer) {.importc: "nomos_da_commitments_free".}
proc nomos_da_reconstruct(
  shares: ptr pointer, share_count: CSizeT, out_data: ptr ptr uint8, out_len: ptr CSizeT
): NomosDaResult {.importc: "nomos_da_reconstruct".}

proc nomos_da_reconstruct_free(
  data: ptr uint8, len: CSizeT
) {.importc: "nomos_da_reconstruct_free".}

proc newEncoder*(columnCount: int): EncoderHandle {.raises: [ValueError].} =
  if columnCount <= 0:
    raise newException(ValueError, "columnCount must be greater than 0")
  let handle = nomos_da_encoder_new(csize_t(columnCount))
  if handle == nil:
    raise newException(ValueError, "Failed to create encoder: " & getLastError())
  EncoderHandle(handle)

proc freeEncoder*(encoder: EncoderHandle) =
  if encoder.pointer != nil:
    nomos_da_encoder_free(encoder.pointer)

proc encode*(
    encoder: EncoderHandle, data: openArray[byte]
): EncodedDataHandle {.raises: [ValueError].} =
  if encoder.pointer == nil:
    raise newException(ValueError, "Encoder handle is null")
  if data.len == 0:
    raise newException(ValueError, "Data length must be greater than 0")
  if data.len mod CHUNK_SIZE != 0:
    raise newException(
      ValueError,
      "Data length (" & $data.len & ") must be a multiple of chunk size (" & $CHUNK_SIZE &
        ")",
    )
  var outHandle: pointer = nil
  let encodeResult = nomos_da_encoder_encode(
    encoder.pointer, unsafeAddr(data[0]), csize_t(data.len), addr outHandle
  )
  if encodeResult != Success:
    raise newException(ValueError, "Encoding failed: " & getLastError())
  if outHandle == nil:
    raise newException(ValueError, "Encoding succeeded but output handle is null")
  EncodedDataHandle(outHandle)

proc freeEncodedData*(encoded: EncodedDataHandle) =
  if encoded.pointer != nil:
    nomos_da_encoded_data_free(encoded.pointer)

proc getData*(encoded: EncodedDataHandle): seq[byte] {.raises: [ValueError].} =
  if encoded.pointer == nil:
    raise newException(ValueError, "Encoded data handle is null")
  var outLen: CSizeT = 0
  var dummy: uint8 = 0
  let result1 = nomos_da_encoded_data_get_data(encoded.pointer, addr dummy, addr outLen)
  if result1 != ErrorInvalidInput:
    raise newException(ValueError, "Failed to get data size: " & getLastError())
  if outLen == 0:
    return @[]
  var output = newSeq[byte](int(outLen))
  var actualLen = outLen
  let result2 =
    nomos_da_encoded_data_get_data(encoded.pointer, addr output[0], addr actualLen)
  if result2 != Success:
    raise newException(ValueError, "Failed to get data: " & getLastError())
  if int(actualLen) < output.len:
    output.setLen(int(actualLen))
  output

func getShareCount*(encoded: EncodedDataHandle): int =
  if encoded.pointer == nil:
    0
  else:
    int(nomos_da_encoded_data_get_share_count(encoded.pointer))

proc getShare*(
    encoded: EncodedDataHandle, index: int
): ShareHandle {.raises: [ValueError].} =
  if encoded.pointer == nil:
    raise newException(ValueError, "Encoded data handle is null")
  if index < 0:
    raise newException(ValueError, "Share index must be non-negative")
  var outShareHandle: pointer = nil
  let shareResult = nomos_da_encoded_data_get_share(
    encoded.pointer, csize_t(index), addr outShareHandle
  )
  if shareResult != Success:
    raise newException(ValueError, "Failed to get share: " & getLastError())
  if outShareHandle == nil:
    raise newException(ValueError, "Share handle is null")
  ShareHandle(outShareHandle)

proc freeShare*(share: ShareHandle) =
  if share.pointer != nil:
    nomos_da_share_free(share.pointer)

func getShareIndex*(share: ShareHandle): int =
  if share.pointer == nil:
    0
  else:
    int(nomos_da_share_get_index(share.pointer))

proc getCommitments*(share: ShareHandle): CommitmentsHandle {.raises: [ValueError].} =
  if share.pointer == nil:
    raise newException(ValueError, "Share handle is null")
  var outCommitmentsHandle: pointer = nil
  let commitmentsResult =
    nomos_da_share_get_commitments(share.pointer, addr outCommitmentsHandle)
  if commitmentsResult != Success:
    raise newException(ValueError, "Failed to get commitments: " & getLastError())
  if outCommitmentsHandle == nil:
    raise newException(ValueError, "Commitments handle is null")
  CommitmentsHandle(outCommitmentsHandle)

proc freeCommitments*(commitments: CommitmentsHandle) =
  if commitments.pointer != nil:
    nomos_da_commitments_free(commitments.pointer)

proc newVerifier*(): VerifierHandle {.raises: [ValueError].} =
  let handle = nomos_da_verifier_new()
  if handle == nil:
    raise newException(ValueError, "Failed to create verifier: " & getLastError())
  VerifierHandle(handle)

proc freeVerifier*(verifier: VerifierHandle) =
  if verifier.pointer != nil:
    nomos_da_verifier_free(verifier.pointer)

proc verify*(
    verifier: VerifierHandle, share: ShareHandle, rowsDomainSize: int
): bool {.raises: [ValueError].} =
  if verifier.pointer == nil:
    raise newException(ValueError, "Verifier handle is null")
  if share.pointer == nil:
    raise newException(ValueError, "Share handle is null")
  if rowsDomainSize <= 0:
    raise newException(ValueError, "Rows domain size must be greater than 0")
  nomos_da_verifier_verify(verifier.pointer, share.pointer, csize_t(rowsDomainSize))

proc reconstruct*(shares: openArray[ShareHandle]): seq[byte] {.raises: [ValueError].} =
  if shares.len == 0:
    raise newException(ValueError, "Share count must be greater than 0")
  for i, share in shares:
    if share.pointer == nil:
      raise newException(ValueError, "Share handle at index " & $i & " is null")
  var sharePtrs = newSeq[pointer](shares.len)
  for i, share in shares:
    sharePtrs[i] = share.pointer
  var outData: ptr uint8 = nil
  var outLen: CSizeT = 0
  let reconstructResult = nomos_da_reconstruct(
    addr sharePtrs[0], csize_t(shares.len), addr outData, addr outLen
  )
  if reconstructResult != Success:
    raise newException(ValueError, "Reconstruction failed: " & getLastError())
  if outData == nil:
    raise newException(ValueError, "Reconstruction succeeded but output data is null")
  if outLen == 0:
    nomos_da_reconstruct_free(outData, outLen)
    raise newException(ValueError, "Reconstructed data length is 0")
  result = newSeq[byte](int(outLen))
  copyMem(addr result[0], outData, int(outLen))
  nomos_da_reconstruct_free(outData, outLen)

# ============================================================================
# Serialization Support (using nim-bincode)
# ============================================================================

import nim_bincode as bc
import stew/endians2

# Re-export BincodeError for convenience
type BincodeError* = bc.BincodeError

type
  SerializableShare* = object
    ## Serializable representation of a share
    index*: uint16
    # Note: Share data itself is opaque and cannot be serialized directly
    # To serialize share data, you would need additional FFI functions

  SerializableEncodedData* = object
    ## Serializable representation of encoded data metadata
    data*: seq[byte]
    shareCount*: uint32

proc serializeUint16*(value: uint16): seq[byte] {.raises: [BincodeError].} =
  ## Serialize a uint16 using bincode
  bc.serialize(@(toBytesLE(value)))

proc deserializeUint16*(data: openArray[byte]): uint16 {.raises: [BincodeError].} =
  ## Deserialize a uint16 using bincode
  let bytes = bc.deserialize(data)
  if bytes.len < 2:
    raise newException(BincodeError, "Cannot deserialize uint16: insufficient data")
  fromBytesLE(uint16, bytes)

proc shareToBytes*(share: ShareHandle): seq[byte] {.raises: [ValueError, BincodeError].} =
  ## Serialize share metadata to bytes
  if share.pointer == nil:
    raise newException(ValueError, "Share handle is null")
  let index = uint16(getShareIndex(share))
  serializeUint16(index)

proc bytesToShare*(data: seq[byte]): SerializableShare {.raises: [BincodeError].} =
  ## Deserialize share metadata from bytes
  let index = deserializeUint16(data)
  SerializableShare(index: index)

proc encodedDataToBytes*(encoded: EncodedDataHandle): seq[byte] {.raises: [ValueError, BincodeError].} =
  ## Serialize encoded data to bytes
  if encoded.pointer == nil:
    raise newException(ValueError, "Encoded data handle is null")
  let data = getData(encoded)
  let shareCount = uint32(getShareCount(encoded))
  # Create a structure with raw bytes for length/count, then serialize the whole thing
  # Use raw bytes for length prefixes (not bincode-wrapped)
  let dataLenBytes = @(toBytesLE(uint32(data.len)))
  let shareCountBytes = @(toBytesLE(shareCount))
  # Combine: length (4 bytes) + data + shareCount (4 bytes)
  let combined = dataLenBytes & data & shareCountBytes
  # Serialize the combined structure with bincode
  bc.serialize(combined)

proc bytesToEncodedData*(data: seq[byte]): SerializableEncodedData {.raises: [BincodeError].} =
  ## Deserialize encoded data from bytes
  let bytes = bc.deserialize(data)
  var offset = 0
  # Read data length (raw 4 bytes, little-endian)
  if bytes.len < offset + 4:
    raise newException(BincodeError, "Insufficient data for data length")
  let dataLen = int(fromBytesLE(uint32, bytes[offset..<offset+4]))
  offset += 4
  # Read data
  if bytes.len < offset + dataLen:
    raise newException(BincodeError, "Insufficient data for encoded data")
  let encodedData = bytes[offset..<offset+dataLen]
  offset += dataLen
  # Read share count (raw 4 bytes, little-endian)
  if bytes.len < offset + 4:
    raise newException(BincodeError, "Insufficient data for share count")
  let shareCount = fromBytesLE(uint32, bytes[offset..<offset+4])
  SerializableEncodedData(data: @encodedData, shareCount: shareCount)

proc serializeData*(data: seq[byte]): seq[byte] {.raises: [BincodeError].} =
  ## Serialize raw data using bincode
  bc.serialize(data)

proc deserializeData*(data: seq[byte]): seq[byte] {.raises: [BincodeError].} =
  ## Deserialize raw data using bincode
  bc.deserialize(data)

proc serializeString*(s: string): seq[byte] {.raises: [BincodeError].} =
  ## Serialize a string using bincode
  bc.serializeString(s)

proc deserializeString*(data: seq[byte]): string {.raises: [BincodeError].} =
  ## Deserialize a string using bincode
  bc.deserializeString(data)

when isMainModule:
  echo "nomos-da Nim wrapper"
  let initResult = nomos_da_init()
  if initResult != Success.cint:
    echo "Initialization failed with code: ", initResult
    let errMsg = getLastError()
    if errMsg.len > 0:
      echo "Error: ", errMsg
    quit(1)
  echo "Initialized successfully"
  echo "Chunk size: ", CHUNK_SIZE, " bytes"
  try:
    let encoder = newEncoder(columnCount = 4)
    defer:
      freeEncoder(encoder)
    var testData = newSeq[byte](CHUNK_SIZE * 2)
    for i in 0 ..< testData.len:
      testData[i] = byte((i mod 256))
    echo "Encoding ", testData.len, " bytes of data..."
    let encoded = encode(encoder, testData)
    defer:
      freeEncodedData(encoded)
    echo "Encoded successfully!"
    echo "Number of shares: ", getShareCount(encoded)
    let retrievedData = getData(encoded)
    echo "Retrieved ", retrievedData.len, " bytes of data"
    if retrievedData == testData:
      echo "✓ Data matches original!"
    else:
      echo "✗ Data mismatch!"
      quit(1)
  except ValueError as e:
    echo "Error: ", e.msg
    quit(1)
  nomos_da_cleanup()
  echo "Cleanup complete"

{.pop.}
