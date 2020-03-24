// Binding muta sdk
export declare namespace BindingSDK {
  export function console_log(message_ptr: u32): void;

  export function read_temp_buffer(start_ptr: u32): void;

  export function alloc_or_recover_map(name: u32): void;

  export function alloc_or_recover_array(name: u32): void;

  export function alloc_or_recover_uint64(name: u32): void;

  export function alloc_or_recover_string(name: u32): void;

  export function alloc_or_recover_bool(name: u32): void;

  export function get_value(name: u32): Uint8Array;

  export function set_value(name: u32, value: u32): void;

  export function get_account_value(name: u32): u32;

  export function set_account_value(name: u32, value: u32): void;

  export function read(service: u32, method: u32, payload: u32): void;

  export function write(service: u32, method: u32, payload: u32): void;
}

// Binding muta store
export declare namespace BindingStore {
  export function get_from_map(name: u32, key: u32): u32;

  export function set_to_map(name: u32, key: u32, value: u32): void;
}
