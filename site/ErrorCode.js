// Ideally this would be a module that exports `ErrorCode`, but this needs to
// be compatible with worker.js, which can't utilize native modules in FireFox
// and Safari yet.
globalThis.ErrorCode = class ErrorCode extends Error {
  static codes = {
    // Default/unknown error.
    0: 'Unknown error occured',

    // Error::InvalidColorspace(_)
    1: 'Invalid image colorspace',

    // Error::InvalidDimensions
    2: 'Invalid image width and/or height',

    // Error::InvalidHeader
    3: 'Invalid QOI image header',

    // Error::InvalidIndex(_)
    4: 'Failed to decode QOI image, invalid index tag',

    // Error::IoError(_)
    5: 'Failed to read or write bytes',

    // Error::UnexpectedEof
    6: 'Unexpected end of file',

    // Error::UnknownTag(_)
    7: 'Failed to decode QOI image, unknown tag',

    // Unknown worker message
    8: 'Received unknown message from worker',

    // `canvas.toBlob()` error
    9: 'Failed to convert canvas to `Blob`',

    // Failed to decode native image and get `ImageData`
    10: 'Failed to get `ImageData` from image',
  };

  /** @type {keyof ErrorCode.codes} */
  code;

  /** @param {number} code */
  constructor(code) {
    super();

    if (code in ErrorCode.codes) {
      this.code = /** @type {keyof ErrorCode.codes} */ (code);
    } else {
      this.code = 0;
    }
  }

  get message() {
    return ErrorCode.codes[this.code];
  }
}
