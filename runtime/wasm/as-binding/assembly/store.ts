import { BindingStore, BindingSDK } from "./binding";
import * as util from "./util";

export class StoreMap {
  private readonly name: string;

  private mapCache: Map<Uint8Array, Uint8Array>;

  constructor(name: string) {
    this.name = name;

    this.mapCache = new Map<Uint8Array, Uint8Array>();
  }

  get(key: Uint8Array): Uint8Array {
    if (this.mapCache.has(key)) {
      return this.mapCache.get(key);
    }

    const valueLen = BindingStore.get_from_map(
      util.getStringPoint(this.name),
      util.getBytesPoint(key)
    );
    const valueBytes = new Uint8Array(valueLen);
    BindingSDK.read_temp_buffer(util.getBytesPoint(valueBytes));

    this.mapCache.set(key, valueBytes);
    return valueBytes;
  }

  set(key: Uint8Array, value: Uint8Array): void {
    this.mapCache.set(key, value);

    BindingStore.set_to_map(
      util.getStringPoint(this.name),
      util.getBytesPoint(key),
      util.getBytesPoint(value)
    );
  }
}
