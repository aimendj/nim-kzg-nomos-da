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

proc nomos_da_max_chunk_size(): CSizeT {.importc: "nomos_da_max_chunk_size".}
func maxChunkSize*(): int = int(nomos_da_max_chunk_size())

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
  let chunkSize = maxChunkSize()
  if data.len mod chunkSize != 0:
    raise newException(
      ValueError,
      "Data length (" & $data.len & ") must be a multiple of chunk size (" & $chunkSize &
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
  echo "Max chunk size: ", maxChunkSize(), " bytes"
  try:
    let encoder = newEncoder(columnCount = 4)
    defer:
      freeEncoder(encoder)
    let chunkSize = maxChunkSize()
    var testData = newSeq[byte](chunkSize * 2)
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
