import { JSONEncoder, JSONDecoder, JSONHandler } from "assemblyscript-json";
import * as util from "./util";

class AccountIDJSONHandler extends JSONHandler {
  public data: string;
  public isServiceAccount: bool;

  setString(name: string, value: string): void {
    if (name === "service") {
      this.data = value;
      this.isServiceAccount = true;
    } else {
      this.data = value;
      this.isServiceAccount = false;
    }
  }
}

export class AccountID {
  private readonly data: string;
  private readonly serviceAccount: bool;

  constructor(data: string, serviceAccount: bool) {
    this.data = data;
    this.serviceAccount = serviceAccount;
  }

  isService(): bool {
    return this.serviceAccount;
  }

  isAddress(): bool {
    return !this.serviceAccount;
  }

  getData(): string {
    return this.data;
  }

  static decode(bytes: Uint8Array): AccountID {
    const handler = new AccountIDJSONHandler();
    const decoder = new JSONDecoder<AccountIDJSONHandler>(handler);
    decoder.deserialize(bytes);

    return new AccountID(handler.data, handler.isServiceAccount);
  }

  encode(): Uint8Array {
    const encoder = new JSONEncoder();

    encoder.pushObject(null);
    if (this.isAddress()) {
      encoder.setString("address", this.getData());
    } else {
      encoder.setString("service", this.getData());
    }
    encoder.popObject();

    return encoder.serialize();
  }
}

export class Hash {
  private readonly data: string;

  constructor(data: string) {
    this.data = data;
  }
}
