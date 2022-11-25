use wasmtime::{Caller, Trap};
use lunatic_common_api::get_memory;

pub(crate) fn get_memory_length<T>(caller: &mut Caller<T>) -> Result<u32, Trap> {
    get_memory(caller)
        .map(|memory| memory.data_size(caller) as u32)
}