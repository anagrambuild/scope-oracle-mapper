#![no_std]

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;

#[cfg(feature = "std")]
extern crate std;

pub mod instruction;
pub use oracle_mapping_state as state;

pinocchio_pubkey::declare_id!("9WM51wrB9xpRzFgYJHocYNnx4DF6G6ee2eB44ZGoZ8vg");
