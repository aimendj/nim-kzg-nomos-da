type
  NomosDaResult* = enum
    Success = 0
    ErrorInvalidInput = -1
    ErrorInternal = -2
    ErrorAllocation = -3

  NomosDaError* = object
    code*: int32
    message*: string

  EncoderHandle* = distinct pointer
  EncodedDataHandle* = distinct pointer
  VerifierHandle* = distinct pointer
  ShareHandle* = distinct pointer
  CSizeT* = csize_t
