#![no_std]

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;

#[cfg(feature = "std")]
extern crate std;

pub mod instruction;
pub use oracle_mapping_state as state;

pinocchio_pubkey::declare_id!("FeDgnLZMJZtq8HzmHc9CLg8YPztDMNyDsZru1Qw3Z19J");
