// Ideally this would be a "module worker" that could utilize a standard
// export, but FireFox and Safari don't support that yet.
globalThis.importScripts('./ErrorCode.js');

/** @type {WasmImports} */
const imports = {
  on_decode_complete: (pointer, width, height, channels, colorspace) => {
    globalThis.dispatchEvent(new CustomEvent('qoi/decode/complete', {
      detail: {
        pointer,
        width,
        height,
        channels,
        colorspace,
      },
    }));
  },

  on_decode_error: (code) => {
    globalThis.dispatchEvent(new CustomEvent('qoi/decode/error', {
      detail: {
        code,
      },
    }));
  },

  on_encode_complete: (pointer, size) => {
    globalThis.dispatchEvent(new CustomEvent('qoi/encode/complete', {
      detail: {
        pointer,
        size,
      },
    }));
  },

  on_encode_error: (code) => {
    globalThis.dispatchEvent(new CustomEvent('qoi/encode/error', {
      detail: {
        code,
      },
    }));
  },
};

globalThis.addEventListener(
  'message',

  /** @param {MessageEvent<LoadMessage>} e */
  (e) => {
    if (e.data.type !== 'load') {
      globalThis.postMessage({ type: 'error', err: new globalThis.ErrorCode(8) });
      console.error('Unknown worker message: ', e.data);
      return;
    }

    WebAssembly.instantiateStreaming(fetch('qoi.wasm'), { env: imports })
      .then((wasm) => {
        globalThis.addEventListener('message', (e) => onMessage(e, wasm));
        globalThis.postMessage({ type: 'ready' });
      })
      .catch((err) => {
        globalThis.postMessage({ type: 'error', err });
      });
  },

  { once: true },
);

// -- Functions ---------------------------------------------------------------

/**
 * @param {MessageEvent<DecodeMessage|EncodeMessage>} e
 * @param {WebAssembly.WebAssemblyInstantiatedSource} wasm
 *
 * @returns {Promise<void>}
 */
async function onMessage(e, wasm) {
  try {
    if (e.data?.type === 'decode') {
      let imageData = await decode(new Uint8Array(e.data.buffer), wasm);
      let transfer = [imageData.data.buffer];

      globalThis.postMessage({ type: 'decodeComplete', imageData }, transfer);
    }

    if (e.data?.type === 'encode') {
      let buffer = await encode(e.data.imageData, wasm);
      let transfer = [buffer];

      globalThis.postMessage({ type: 'encodeComplete', buffer }, transfer);
    }
  } catch (err) {
    globalThis.postMessage({ type: 'error', err });
  }
}

/**
 * @param {Uint8Array} data
 * @param {WebAssembly.WebAssemblyInstantiatedSource} wasm 
 *
 * @returns {Promise<ImageData>}
 */
 function decode(data, wasm) {
  let {
    qoi_dealloc,
    qoi_image_decode,
  } = /** @type {WasmExports} */ (wasm.instance.exports);

  return new Promise((resolve, reject) => {
    let copyPointer = copyIntoWasm(data, wasm);

    globalThis.addEventListener(
      'qoi/decode/complete',

      /** @param {DecodeCompleteEvent} e */
      (e) => {
        let {
          detail: {
            pointer,
            width,
            height,
            channels,
          },
        } = e;
  
        let length = width * height * channels;
        let buffer = copyFromWasm(pointer, length, wasm);
  
        qoi_dealloc(copyPointer, data.byteLength);
        qoi_dealloc(pointer, length);

        let imageData = new ImageData(
          new Uint8ClampedArray(buffer),
          width,
          height,
        );

        resolve(imageData);
      },

      { once: true },
    );

    globalThis.addEventListener(
      'qoi/decode/error',

      /** @param {DecodeErrorEvent} e */
      (e) => {
        let { detail: { code } } = e;

        qoi_dealloc(copyPointer, data.byteLength);
        reject(new globalThis.ErrorCode(code));
      },

      { once: true },
    );

    qoi_image_decode(copyPointer, data.byteLength);
  });
}

/**
 * @param {ImageData} imageData
 * @param {WebAssembly.WebAssemblyInstantiatedSource} wasm
 *
 * @returns {Promise<ArrayBuffer>}
 */
function encode(imageData, wasm) {
  let {
    qoi_dealloc,
    qoi_image_encode,
  } = /** @type {WasmExports} */ (wasm.instance.exports);

  return new Promise((resolve, reject) => {
    let copyPointer = copyIntoWasm(imageData.data, wasm);

    globalThis.addEventListener(
      'qoi/encode/complete',

      /** @param {EncodeCompleteEvent} e */
      (e) => {
        let {
          detail: {
            pointer,
            size,
          },
        } = e;
  
        let buffer = copyFromWasm(pointer, size, wasm);
  
        qoi_dealloc(copyPointer, imageData.data.byteLength);
        qoi_dealloc(pointer, size);
        resolve(buffer);
      },

      { once: true },
    );

    globalThis.addEventListener(
      'qoi/encode/error',

      /** @param {EncodeErrorEvent} e */
      (e) => {
        let { detail: { code } } = e;

        qoi_dealloc(copyPointer, imageData.data.byteLength);
        reject(new globalThis.ErrorCode(code));
      },

      { once: true },
    );

    // Assuming that `imageData` came from a canvas, the colorspace will
    // pretty much always be sRGB.
    let colorspace = 0;

    // If running in a browser that happens to support newer colorspace
    // features, use `imageData.colorSpace`.
    // @ts-expect-error
    if (('colorSpace' in imageData) && imageData.colorSpace !== 'srgb') {
      colorspace = 1;
    }

    qoi_image_encode(
      imageData.width,
      imageData.height,
      colorspace,
      copyPointer,
      imageData.data.byteLength,
    );
  });
}

/**
 * Copies bytes out of the `wasm` instance's memory buffer starting at
 * `pointer` up to `length`.
 *
 * @param {number} pointer
 * @param {number} length
 * @param {WebAssembly.WebAssemblyInstantiatedSource} wasm
 *
 * @returns {ArrayBuffer}
 */
 function copyFromWasm(pointer, length, wasm) {
  let { memory } = /** @type {WasmExports} */ (wasm.instance.exports);

  return memory.buffer.slice(pointer, pointer + length);
}

/**
 * Copies the given bytes into the `wasm` instance's memory buffer and returns
 * a pointer to its location.
 *
 * @param {Uint8Array|Uint8ClampedArray} data
 * @param {WebAssembly.WebAssemblyInstantiatedSource} wasm
 *
 * @returns {number}
 */
 function copyIntoWasm(data, wasm) {
  let { qoi_malloc, memory } = /** @type {WasmExports} */ (wasm.instance.exports);

  let pointer = qoi_malloc(data.byteLength);
  let slice = new Uint8Array(memory.buffer, pointer, data.byteLength);

  slice.set(data);

  return pointer;
}

// -- Types -------------------------------------------------------------------

/**
 * @typedef {{
 *   on_decode_complete: (
 *     pointer: number,
 *     width: number,
 *     height: number,
 *     channels: number,
 *     colorspace: number,
 *   ) => void;
 *   on_decode_error: (code: number) => void;
 *   on_encode_complete: (pointer: number, size: number) => void;
 *   on_encode_error: (code: number) => void;
 * }} WasmImports
 */

/**
 * @typedef {{
 *   memory: WebAssembly.Memory;
 *   qoi_dealloc: (pointer: number, size: number) => void;
 *   qoi_image_decode: (pointer: number, size: number) => void;
 *   qoi_image_encode: (
 *     width: number,
 *     height: number,
 *     colorspace: number,
 *     pointer: number,
 *     size: number,
 *   ) => void;
 *   qoi_malloc: (size: number) => number;
 * }} WasmExports
 */

/**
 * @typedef {CustomEvent<{
 *   pointer: number;
 *   width: number;
 *   height: number;
 *   channels: number;
 *   colorspace: number;
 * }>} DecodeCompleteEvent
 */

/** @typedef {CustomEvent<{ code: number }>} DecodeErrorEvent */

/**
 * @typedef {CustomEvent<{
 *   pointer: number;
 *   size: number;
 * }>} EncodeCompleteEvent
 */

/** @typedef {CustomEvent<{ code: number }>} EncodeErrorEvent */

/** @typedef {{ type: "load" }} LoadMessage */
/** @typedef {{ type: "ready" }} ReadyMessage */
/** @typedef {{ type: "error"; err: Error; }} ErrorMessage */
/** @typedef {{ type: "decode"; buffer: ArrayBuffer }} DecodeMessage */
/** @typedef {{ type: "encode"; imageData: ImageData }} EncodeMessage */
/** @typedef {{ type: "decodeComplete"; imageData: ImageData; }} DecodeCompleteMessage */
/** @typedef {{ type: "encodeComplete"; buffer: ArrayBuffer; }} EncodeCompleteMessage */
