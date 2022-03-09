// Ideally this would be a module that exports `ErrorCode`, but this needs to
// be compatible with worker.js, which can't utilize native modules in FireFox
// and Safari yet.
import './ErrorCode.js';

let controls = view();

loadWorker()
  .then((worker) => {
    controls.setOnInputChange((e) => {
      onInputChange({
        event: e,
        update: controls.update,
        worker,
      });
    });

    controls.update({ status: 'ready' });
  })
  .catch((error) => {
    controls.update({ status: 'error', error });
  });

// -- Functions ---------------------------------------------------------------

/**
 * @returns {Promise<Worker>}
 */
function loadWorker() {
  return new Promise((resolve, reject) => {
    let worker = new Worker('worker.js');

    worker.addEventListener(
      'message',

      /** @param {MessageEvent<ReadyMessage|ErrorMessage>} e */
      (e) => {
        if (e.data.type === 'ready') {
          return resolve(worker);
        }

        if (e.data.type === 'error') {
          return reject(e.data.err);
        }

        reject(new globalThis.ErrorCode(8));
        console.error('Unknown worker message: ', e.data);
      },

      { once: true },
    );

    worker.postMessage({ type: 'load' });
  });
}

/**
 * @returns {ViewControls}
 */
 function view() {
  let container = /** @type {HTMLElement} */ (document.getElementById('converter'));
  let input = /** @type {HTMLInputElement} */ (container.querySelector('.imageInput input'));
  let image = /** @type {HTMLImageElement} */ (container.querySelector('.imageInfo img'));
  let link = /** @type {HTMLAnchorElement} */ (container.querySelector('.imageInfoLink'));
  let errorStatus = /** @type {HTMLElement} */ (container.querySelector('.statusBarMessage[data-status="error"]'));

  /** @type {((e: Event) => void)|null} */
  let onInputChange = null;

  input.addEventListener('change', (e) => {
    onInputChange?.(e);
  });

  let updateFns = {
    /** @param {string} value */
    downloadName: (value) => {
      image.alt = value;
      link.download = value;
      link.textContent = value;

      try {
        link.focus();
      } catch (_) {};
    },

    /** @param {string} value */
    downloadSize: (value) => {
      if (value) {
        link.textContent = `${link.textContent} (${value} MiB)`;
      } else {
        link.textContent = `${link.textContent}`;
      }
    },

    /** @param {string} value */
    downloadUrl: (value) => {
      if (link.href) {
        URL.revokeObjectURL(link.href);
      }

      link.href = value;
    },

    /** @param {Error} error */
    error: (error) => {
      errorStatus.textContent = `error - ${error.message}`;
      console.error(error);
    },

    /** @param {string} value */
    imageUrl: (value) => {
      if (image.src) {
        URL.revokeObjectURL(image.src);
      }

      image.src = value;
    },

    /** @param {string} value */
    status: (value) => {
      if (value === 'ready') {
        input.removeAttribute('disabled');
      } else {
        input.setAttribute('disabled', 'true');
      }

      document.documentElement.setAttribute('data-status', value);
    },
  };

  return {
    setOnInputChange: (callback) => {
      onInputChange = callback;
    },

    /** @param {ViewProps} props */
    update: (props) => {
      for (let [ key, value ] of Object.entries(props)) {
        // @ts-expect-error
        updateFns[key]?.(value);
      }
    },
  };
}

/**
 * @param {{
 *   event: Event;
 *   update: (props: ViewProps) => void;
 *   worker: Worker;
 * }} args
 *
 * @returns {Promise<void>}
 */
async function onInputChange({ event, update, worker }) {
  event.preventDefault();

  let { files } = /** @type {HTMLInputElement} */ (event.target);

  if (!files) {
    return;
  }

  let file = files.item(0);

  if (!file) {
    return;
  }

  update({
    imageUrl: '',
    downloadName: '',
    downloadSize: '',
    downloadUrl: '',
    status: 'processing',
  });

  try {
    let result = await processImageFile(file, worker);

    update({ status: 'ready', ...result });
  } catch (error) {
    update({
      status: 'error',
      error: error instanceof Error
        ? error
        : new globalThis.ErrorCode(0),
    });
  }
}

/**
 * Processes the provided image file in one of two ways:
 *
 * 1. If the file is a QOI encoded image, it gets decoded and the underlying
 * image data is returned. From there the image data is rendered onto a canvas
 * and is made available for download as a PNG.
 *
 * 2. If the file is any other encoded image format supported by the browser,
 * it first gets decoded and then re-encoded as a QOI image, which is made
 * available for download.
 *
 * @param {File} file 
 * @param {Worker} worker
 *
 * @returns {Promise<ViewProps>}
 */
async function processImageFile(file, worker) {
  let fileBuffer = await file.arrayBuffer();
  let fileName = file.name.split('.').slice(0, -1).join('.');

  if (isQoiImage(fileBuffer)) {
    let imageData = await qoiDecodeImage(worker, fileBuffer);
    let blob = await blobFromImageData(imageData);
    let url = URL.createObjectURL(blob);

    return {
      imageUrl: url,
      downloadName: `${fileName}.png`,
      downloadSize: bytesToMebibytes(blob.size),
      downloadUrl: url,
    };
  }

  let imageData = await decodeNativeImage(file);

  if (!imageData) {
    return { status: 'error', error: new globalThis.ErrorCode(10) };
  }

  let blob = await blobFromImageData(imageData);
  let buffer = await qoiEncodeImage(worker, imageData);

  return {
    imageUrl: URL.createObjectURL(blob),
    downloadName: `${fileName}.qoi`,
    downloadSize: bytesToMebibytes(buffer.byteLength),
    downloadUrl: URL.createObjectURL(new Blob([buffer], { type: 'image/qoi' })),
  };
}

/**
 * Decodes the QOI encoded image buffer into an `ImageData` instance.
 *
 * @param {Worker} worker
 * @param {ArrayBuffer} buffer
 *
 * @returns {Promise<ImageData>}
 */
async function qoiDecodeImage(worker, buffer) {
  return new Promise((resolve, reject) => {
    worker.addEventListener(
      'message',

      /** @param {MessageEvent<DecodeCompleteMessage|ErrorMessage>} e */
      (e) => {
        if (e.data.type === 'decodeComplete') {
          return resolve(e.data.imageData);
        }

        if (e.data.type === 'error') {
          return reject(e.data.err);
        }

        reject(new globalThis.ErrorCode(8));
        console.error('Unknown worker message: ', e.data);
      },

      { once: true },
    );

    worker.postMessage({ type: 'decode', buffer }, [buffer]);
  });
}

/**
 * Encodes the provided image data as a QOI image.
 *
 * @param {Worker} worker
 * @param {ImageData} imageData
 *
 * @returns {Promise<ArrayBuffer>}
 */
async function qoiEncodeImage(worker, imageData) {
  return new Promise((resolve, reject) => {
    worker.addEventListener(
      'message',

      /** @param {MessageEvent<EncodeCompleteMessage|ErrorMessage>} e */
      (e) => {
        if (e.data.type === 'encodeComplete') {
          return resolve(e.data.buffer);
        }

        if (e.data.type === 'error') {
          return reject(e.data.err);
        }

        reject(new globalThis.ErrorCode(8));
        console.error('Unknown worker message: ', e.data);
      },

      { once: true },
    );

    worker.postMessage({ type: 'encode', imageData }, [imageData.data.buffer]);
  });
}

/**
 * Decodes the provided image file and returns its width, height, and a buffer
 * of the pixel data.
 *
 * Assumes the provided file represents an encoded image that the browser
 * natively supports.
 *
 * It would be more ideal to do this work off the main thread in a worker, but
 * there isn't a well-supported cross browser way to do so at the moment.
 *
 * @param {File} file
 *
 * @returns {Promise<ImageData|null>}
 */
async function decodeNativeImage(file) {
  let url = URL.createObjectURL(file);
  let image = new Image();

  image.src = url;
  await image.decode();

  let data = imageDataFromImage(image);

  URL.revokeObjectURL(url);

  return data;
}

/**
 * Takes the provided HTML Image and renders it on a HTML Canvas so that the
 * pixel data can be extracted and returned along with the image's dimensions.
 *
 * @param {HTMLImageElement} image
 *
 * @returns {ImageData|null}
 */
function imageDataFromImage(image) {
  let canvas = document.createElement('canvas');

  canvas.width = image.naturalWidth;
  canvas.height = image.naturalHeight;

  let ctx = /** @type {CanvasRenderingContext2D} */ (canvas.getContext('2d'));

  ctx.drawImage(image, 0, 0);

  return ctx.getImageData(0, 0, image.naturalWidth, image.naturalHeight);
}

/**
 * Takes the provided image data and renders it into a canvas. A `Blob`
 * representing the data as an image of the specified type and quality is
 * returned.
 *
 * @param {ImageData} imageData
 *
 * @returns {Promise<Blob>}
 */
function blobFromImageData(imageData, type = 'image/png', quality = 1) {
  return new Promise((resolve, reject) => {
    let canvas = document.createElement('canvas');

    canvas.width = imageData.width;
    canvas.height = imageData.height;

    let ctx = /** @type {CanvasRenderingContext2D} */ (canvas.getContext('2d'));

    ctx.putImageData(imageData, 0, 0);

    canvas.toBlob(
      (result) => result
        ? resolve(result)
        : reject(new globalThis.ErrorCode(9)),
      type,
      quality,
    );
  });
}

/**
 * Unicode code points for the string "qoif" as bytes.
 */
const qoiBytesMagic = [
  'q'.codePointAt(0),
  'o'.codePointAt(0),
  'i'.codePointAt(0),
  'f'.codePointAt(0),
];

/**
 * QOI end marker bytes.
 */
const qoiBytesEnd = [0, 0, 0, 0, 0, 0, 0, 1];

/**
 * Determines if `buffer` represents a QOI encoded image by asserting that the
 * first four bytes make up the magic bytes "qoif", and that the last eight
 * bytes match the QOI end marker.
 *
 * @param {ArrayBuffer} buffer
 *
 * @returns {boolean}
 */
function isQoiImage(buffer) {
  let magicBytesSlice = new Uint8Array(buffer, 0, qoiBytesMagic.length);

  for (let [ index, magicByte ] of qoiBytesMagic.entries()) {
    if (magicBytesSlice[index] !== magicByte) {
      return false;
    }
  }

  let endMarkerSlice = new Uint8Array(buffer, buffer.byteLength - qoiBytesEnd.length);

  for (let [ index, endByte ] of qoiBytesEnd.entries()) {
    if (endMarkerSlice[index] !== endByte) {
      return false;
    }
  }

  return true;
}

/**
 * @param {number} numBytes
 * @param {number} [precision]
 *
 * @returns {string}
 */
 function bytesToMebibytes(numBytes, precision = 2) {
  return (numBytes / Math.pow(1024, 2)).toFixed(precision);
}

// -- Types -------------------------------------------------------------------

/** @typedef {"error"|"loading"|"processing"|"ready"} ViewStatus */

/**
 * @typedef {Partial<{
 *   downloadName: string;
 *   downloadSize: string;
 *   downloadUrl: string;
 *   error: Error;
 *   imageUrl: string;
 *   status: ViewStatus;
 * }>} ViewProps
 */

/**
 * @typedef {{
 *   setOnInputChange: (callback: (e: Event) => any) => void;
 *   update: (props: ViewProps) => void;
 * }} ViewControls
 */
