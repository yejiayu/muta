use wasmer_runtime::{func, imports, Ctx, ImportObject};

use protocol::traits::ServiceSDK;

use crate::binding_proxy::BindingProxy;

// TODO(yejiayu): use macro.
pub fn build_imports<SDK: 'static + ServiceSDK>(proxy: BindingProxy<SDK>) -> ImportObject {
    let abort_proxy = proxy.clone();
    let console_log_proxy = proxy.clone();
    let alloc_or_recover_map_proxy = proxy.clone();
    let get_from_map_proxy = proxy.clone();
    let set_to_map_proxy = proxy.clone();
    let read_temp_buffer_proxy = proxy.clone();

    let import_object = imports! {
        "env" => {
            "abort" => func!(move |ctx: &mut Ctx, message_ptr: u32, filename_ptr: u32, lineNumber: u32, columnNumber: u32| {
                let mut proxy = abort_proxy.clone();
                proxy.abort(ctx,message_ptr, filename_ptr, lineNumber, columnNumber)
            }),
        },
        "binding" => {
            // SDK
            "BindingSDK.console_log" => func!(move |ctx: &mut Ctx, message_ptr: u32| {
                let mut proxy = console_log_proxy.clone();
                proxy.console_log(ctx,message_ptr)
            }),

            "BindingSDK.alloc_or_recover_map" => func!(move |ctx: &mut Ctx, name_ptr: u32| {
                let mut proxy = alloc_or_recover_map_proxy.clone();
                proxy.alloc_or_recover_map(ctx, name_ptr)
            }),
            "BindingSDK.read_temp_buffer" => func!(move |ctx: &mut Ctx, ptr: u32| {
                let mut proxy = read_temp_buffer_proxy.clone();
                proxy.read_temp_buffer(ctx, ptr)
            }),

            // Store
            "BindingStore.get_from_map" => func!(move |ctx: &mut Ctx, name_ptr: u32, key_ptr: u32| -> u32 {
                let mut proxy = get_from_map_proxy.clone();
                proxy.get_from_map(ctx, name_ptr, key_ptr)
            }),
            "BindingStore.set_to_map" => func!(move |ctx: &mut Ctx, name_ptr: u32, key_ptr: u32, value_ptr: u32| {
                let mut proxy = set_to_map_proxy.clone();
                proxy.set_to_map(ctx, name_ptr, key_ptr, value_ptr)
            }),
        },
    };

    import_object
}
