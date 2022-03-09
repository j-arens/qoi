import {
  DecodeCompleteEvent,
  DecodeErrorEvent,
  EncodeCompleteEvent,
  EncodeErrorEvent,
} from "./worker";

declare global {
  interface WindowEventMap {
      "qoi/decode/complete": DecodeCompleteEvent;
      "qoi/decode/error": DecodeErrorEvent;
      "qoi/encode/complete": EncodeCompleteEvent;
      "qoi/encode/error": EncodeErrorEvent;
  };
}
