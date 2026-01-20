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
    let msg = (if operation.len > 0: operation & " failed: " else: "") &
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
proc nomos_da_share_get_index(share_handle: pointer): uint16 {.
  importc: "nomos_da_share_get_index"
.}

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
  if encoded.pointer == nil: 0
  else: int(nomos_da_encoded_data_get_share_count(encoded.pointer))

proc getShare*(encoded: EncodedDataHandle, index: int): ShareHandle {.
  raises: [ValueError]
.} =
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
  if share.pointer == nil: 0
  else: int(nomos_da_share_get_index(share.pointer))

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
