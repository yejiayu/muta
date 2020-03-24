import { BindingSDK } from "./binding";
import { StoreMap } from "./store";
import * as util from "./util";

export class SDK {
  static alloc_or_recover_map(name: string): StoreMap {
    BindingSDK.alloc_or_recover_map(util.getStringPoint(name));

    return new StoreMap(name);
  }
}
