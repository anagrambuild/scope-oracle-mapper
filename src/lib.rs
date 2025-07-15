#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;

#[cfg(feature = "std")]
extern crate std;

pub mod error;
pub mod instruction;
pub mod state;

pinocchio_pubkey::declare_id!("4Yg8cVpMUqbvyb9qF13mZarqvNCdDC9uVJeeDvSCLVSK");
