use wasmtime::{Caller, Memory};

pub(crate) fn get_memory<T>(caller: &mut Caller<T>) -> Memory {
    caller
        .get_export("memory")
        .unwrap()
        .into_memory()
        .unwrap()
}

pub(crate) fn get_memory_length<T>(caller: &mut Caller<T>) -> u32 {
    get_memory(caller).data_size(caller) as u32
}