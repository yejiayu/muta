import { JSONDecoder, JSONHandler } from "assemblyscript-json";
import { BindingSDK } from "./binding";
import * as util from "./util";

class ContextHandler extends JSONHandler {
  nonce: string = "";
  serviceName: string = "";
  serviceMethod: string = "";
  servicePayload: string = "";
  cyclesLimit: u64 = 0;
  cyclesPrice: u64 = 0;
  height: u64 = 0;
  timestamp: u64 = 0;
  cyclesUsed: u64 = 0;

  setString(name: string, value: string): void {
    if (name == "nonce") {
      this.nonce = value;
    } else if (name == "service_name") {
      this.serviceName = value;
    } else if (name == "service_method") {
      this.serviceMethod = value;
    } else if (name == "service_payload") {
      this.servicePayload = value;
    }
  }

  setInteger(name: string, value: i64): void {
    if (name == "cycles_limit") {
      this.cyclesLimit = value;
    } else if (name == "cycles_price") {
      this.cyclesPrice = value;
    } else if (name == "height") {
      this.height = value;
    } else if (name == "timestamp") {
      this.timestamp = value;
    } else if (name == "cycles_used") {
      this.cyclesUsed = value;
    }
  }
}

export class Context {
  private readonly nonce: string;
  private readonly serviceName: string;
  private readonly serviceMethod: string;
  private readonly servicePayload: string;
  private readonly cyclesLimit: u64;
  private readonly cyclesPrice: u64;
  private readonly height: u64;
  private readonly timestamp: u64;

  private cyclesUsed: u64;

  constructor(
    nonce: string,
    cyclesLimit: u64,
    cyclesPrice: u64,
    height: u64,
    timestamp: u64,
    serviceName: string,
    serviceMethod: string,
    servicePayload: string,
    cyclesUsed: u64
  ) {
    this.nonce = nonce;
    this.cyclesLimit = cyclesLimit;
    this.cyclesPrice = cyclesPrice;
    this.height = height;
    this.timestamp = timestamp;
    this.serviceName = serviceName;
    this.serviceMethod = serviceMethod;
    this.servicePayload = servicePayload;
    this.cyclesUsed = cyclesUsed;
  }

  static fromBytes(contextBytesLen: u32): Context {
    const contextBytes = new Uint8Array(contextBytesLen);
    BindingSDK.read_temp_buffer(util.getBytesPoint(contextBytes));

    const handler = new ContextHandler();
    const decoder = new JSONDecoder<ContextHandler>(handler);
    decoder.deserialize(contextBytes);

    return new Context(
      handler.nonce,
      handler.cyclesLimit,
      handler.cyclesPrice,
      handler.height,
      handler.timestamp,
      handler.serviceName,
      handler.serviceMethod,
      handler.servicePayload,
      handler.cyclesUsed
    );
  }

  getPayload(): string {
    return this.servicePayload;
  }

  getNonce(): string {
    return this.nonce;
  }

  getServiceName(): string {
    return this.serviceName;
  }

  getServiceMethod(): string {
    return this.serviceMethod;
  }

  getCyclesLimit(): u64 {
    return this.cyclesLimit;
  }

  getCyclesPrice(): u64 {
    return this.cyclesPrice;
  }

  getGeight(): u64 {
    return this.height;
  }

  getTimestamp(): u64 {
    return this.timestamp;
  }

  getCyclesUsed(): u64 {
    return this.cyclesUsed;
  }
}
