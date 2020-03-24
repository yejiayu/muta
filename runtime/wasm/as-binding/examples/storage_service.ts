import * as asBinding from "../assembly";

class StorageService {
  set_storage(ctx: asBinding.Context): u32 {
    const payload = ctx.getPayload();

    const map = asBinding.SDK.alloc_or_recover_map("storage-map");
    map.set(asBinding.stringToBytes("key"), asBinding.stringToBytes(payload));

    return asBinding.ok("key");
  }

  get_storage(ctx: asBinding.Context): u32 {
    const map = asBinding.SDK.alloc_or_recover_map("storage-map");

    const value = map.get(asBinding.stringToBytes("key"));

    return asBinding.ok(asBinding.bytesToString(value));
  }
}

export function write(contextBytesLen: u32): u32 {
  const ctx = asBinding.Context.fromBytes(contextBytesLen);
  const method = ctx.getServiceMethod();

  const storageService = new StorageService();

  if (method == "set_storage") {
    return storageService.set_storage(ctx);
  } else {
    return asBinding.err(404, "NotFound");
  }
}

export function read(contextBytesLen: u32): u32 {
  const ctx = asBinding.Context.fromBytes(contextBytesLen);
  const method = ctx.getServiceMethod();

  const storageService = new StorageService();

  if (method == "get_storage") {
    return storageService.get_storage(ctx);
  } else {
    return asBinding.err(404, "NotFound");
  }
}
