import { JSONEncoder } from "assemblyscript-json";
import { BindingSDK } from "./binding";

export function getBytesPoint(arr: Uint8Array): u32 {
  // @ts-ignore
  return arr.dataStart as u32;
}

export function getStringPoint(str: string): u32 {
  return getBytesPoint(stringToBytes(str));
}

export function stringToBytes(str: string): Uint8Array {
  const buffer = String.UTF8.encode(str);
  return Uint8Array.wrap(buffer);
}

export function console_log(message: string): void {
  BindingSDK.console_log(getBytesPoint(stringToBytes(message)));
}

export function bytesToString(bytes: Uint8Array): string {
  return String.UTF8.decode(bytes.buffer);
}

export function ok(data: string): u32 {
  const encoder = new JSONEncoder();
  encoder.pushObject(null);
  encoder.pushObject("success");
  encoder.setString("data", data);
  encoder.popObject();
  encoder.popObject();

  return getBytesPoint(encoder.serialize());
}

export function err(code: u32, message: string): u32 {
  const encoder = new JSONEncoder();
  encoder.pushObject(null);
  encoder.pushObject("error");
  encoder.setString("message", message);
  encoder.setInteger("code", code);
  encoder.popObject();
  encoder.popObject();

  return getBytesPoint(encoder.serialize());
}
