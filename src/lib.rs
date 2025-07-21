#![no_std]

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;

#[cfg(feature = "std")]
extern crate std;

pub mod error;
pub mod instruction;
pub mod state;

pinocchio_pubkey::declare_id!("HeyqQW2AYdG9F8d25UZYTwV6SjEXbwwxngSrhem1D1Ww");
