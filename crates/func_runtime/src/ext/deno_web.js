// https://github.com/denoland/deno/tree/main/ext/web

import * as infra from "ext:deno_web/00_infra.js";
import * as DOMException from "ext:deno_web/01_dom_exception.js";
import * as mimesniff from "ext:deno_web/01_mimesniff.js";
import * as event from "ext:deno_web/02_event.js";
import * as structuredClone from "ext:deno_web/02_structured_clone.js";
import * as timers from "ext:deno_web/02_timers.js";
import * as abortSignal from "ext:deno_web/03_abort_signal.js";
import * as globalInterfaces from "ext:deno_web/04_global_interfaces.js";
import * as base64 from "ext:deno_web/05_base64.js";
import * as streams from "ext:deno_web/06_streams.js";
import * as encoding from "ext:deno_web/08_text_encoding.js";
import * as file from "ext:deno_web/09_file.js";
import * as fileReader from "ext:deno_web/10_filereader.js";
import * as location from "ext:deno_web/12_location.js";
import * as messagePort from "ext:deno_web/13_message_port.js";
import * as compression from "ext:deno_web/14_compression.js";
import * as performance from "ext:deno_web/15_performance.js";
import * as imageData from "ext:deno_web/16_image_data.js";

Object.defineProperty(globalThis, "AbortController", {
  value: abortSignal.AbortController,
  enumerable: false,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "AbortSignal", {
  value: abortSignal.AbortSignal,
  enumerable: false,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "Blob", {
  value: file.Blob,
  enumerable: false,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "ByteLengthQueuingStrategy", {
  value: streams.ByteLengthQueuingStrategy,
  enumerable: false,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "CloseEvent", {
  value: event.CloseEvent,
  enumerable: false,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "CompressionStream", {
  value: compression.CompressionStream,
  enumerable: false,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "CountQueuingStrategy", {
  value: streams.CountQueuingStrategy,
  enumerable: false,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "CustomEvent", {
  value: event.CustomEvent,
  enumerable: false,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "DecompressionStream", {
  value: compression.DecompressionStream,
  enumerable: false,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "DOMException", {
  value: DOMException.DOMException,
  enumerable: false,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "ErrorEvent", {
  value: event.ErrorEvent,
  enumerable: false,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "Event", {
  value: event.Event,
  enumerable: false,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "EventTarget", {
  value: event.EventTarget,
  enumerable: false,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "File", {
  value: file.File,
  enumerable: false,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "FileReader", {
  value: fileReader.FileReader,
  enumerable: false,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "MessageEvent", {
  value: event.MessageEvent,
  enumerable: false,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "Performance", {
  value: performance.Performance,
  enumerable: false,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "PerformanceEntry", {
  value: performance.PerformanceEntry,
  enumerable: false,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "PerformanceMark", {
  value: performance.PerformanceMark,
  enumerable: false,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "PerformanceMeasure", {
  value: performance.PerformanceMeasure,
  enumerable: false,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "PromiseRejectionEvent", {
  value: event.PromiseRejectionEvent,
  enumerable: false,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "ProgressEvent", {
  value: event.ProgressEvent,
  enumerable: false,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "ReadableStream", {
  value: streams.ReadableStream,
  enumerable: false,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "ReadableStreamDefaultReader", {
  value: streams.ReadableStreamDefaultReader,
  enumerable: false,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "TextDecoder", {
  value: encoding.TextDecoder,
  enumerable: false,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "TextEncoder", {
  value: encoding.TextEncoder,
  enumerable: false,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "TextDecoderStream", {
  value: encoding.TextDecoderStream,
  enumerable: false,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "TextEncoderStream", {
  value: encoding.TextEncoderStream,
  enumerable: false,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "TransformStream", {
  value: streams.TransformStream,
  enumerable: false,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "MessageChannel", {
  value: messagePort.MessageChannel,
  enumerable: false,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "MessagePort", {
  value: messagePort.MessagePort,
  enumerable: false,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "WritableStream", {
  value: streams.WritableStream,
  enumerable: false,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "WritableStreamDefaultWriter", {
  value: streams.WritableStreamDefaultWriter,
  enumerable: false,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "WritableStreamDefaultController", {
  value: streams.WritableStreamDefaultController,
  enumerable: false,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "ReadableByteStreamController", {
  value: streams.ReadableByteStreamController,
  enumerable: false,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "ReadableStreamBYOBReader", {
  value: streams.ReadableStreamBYOBReader,
  enumerable: false,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "ReadableStreamBYOBRequest", {
  value: streams.ReadableStreamBYOBRequest,
  enumerable: false,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "ReadableStreamDefaultController", {
  value: streams.ReadableStreamDefaultController,
  enumerable: false,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "TransformStreamDefaultController", {
  value: streams.TransformStreamDefaultController,
  enumerable: false,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "ImageData", {
  value: imageData.ImageData,
  enumerable: false,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "atob", {
  value: base64.atob,
  enumerable: true,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "btoa", {
  value: base64.btoa,
  enumerable: true,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "clearInterval", {
  value: timers.clearInterval,
  enumerable: true,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "clearTimeout", {
  value: timers.clearTimeout,
  enumerable: true,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "performance", {
  value: performance.performance,
  enumerable: true,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "reportError", {
  value: event.reportError,
  enumerable: true,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "setInterval", {
  value: timers.setInterval,
  enumerable: true,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "setTimeout", {
  value: timers.setTimeout,
  enumerable: true,
  configurable: true,
  writable: true,
});

Object.defineProperty(globalThis, "structuredClone", {
  value: messagePort.structuredClone,
  enumerable: true,
  configurable: true,
  writable: true,
});
