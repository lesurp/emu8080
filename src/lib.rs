#![feature(split_at_checked)]
#![feature(generic_arg_infer)]

pub mod cpu_state;
pub mod in_out;
pub mod interrupts;
pub mod op_code;

#[cfg(target_arch = "wasm32")]
mod wasm;
