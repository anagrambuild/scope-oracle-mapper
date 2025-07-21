#![no_std]

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;

#[cfg(feature = "std")]
extern crate std;

pub mod error;
pub mod instruction;
pub mod state;

pinocchio_pubkey::declare_id!("Fhjf6d3Dj5Y4a5pGq5AGXgZ5ARasoob1a6WF1X2CaN2o");
