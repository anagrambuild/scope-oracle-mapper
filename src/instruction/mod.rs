use pinocchio::{program_error::ProgramError, pubkey::Pubkey};

pub mod add_mapping;
pub mod close;
pub mod initialize;

pub use add_mapping::*;
pub use close::*;
pub use initialize::*;
use pinocchio_pubkey::pubkey;

const OWNER_PUB_KEY: Pubkey = pubkey!("3hPmQsxMb4buU1PozSqMS7wni14JoP5kmPA9UTpJnerb");

#[repr(u8)]
pub enum InstructionSet {
    InitializeState,
    AddMapping,
    CloseMapping,
}

pub trait IntoBytes {
    /// Converts the implementing type into a byte slice.
    fn into_bytes(&self) -> Result<&[u8], ProgramError>;
}

impl TryFrom<&u8> for InstructionSet {
    type Error = ProgramError;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match *value {
            0 => Ok(InstructionSet::InitializeState),
            1 => Ok(InstructionSet::AddMapping),
            2 => Ok(InstructionSet::CloseMapping),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}

mod idl_gen {
    use super::InitializeRegistryIxData;

    #[derive(shank::ShankInstruction)]
    enum _InstructionSet {
        #[account(0, writable, signer, name = "payer_acc", desc = "Fee payer account")]
        #[account(1, writable, name = "state_acc", desc = "New State account")]
        #[account(2, name = "sysvar_rent_acc", desc = "Sysvar rent account")]
        #[account(3, name = "system_program_acc", desc = "System program account")]
        InitializeState(InitializeRegistryIxData),
        #[account(0, writable, signer, name = "payer_acc", desc = "Fee payer account")]
        #[account(1, writable, name = "state_acc", desc = "State account")]
        AddMapping,
        #[account(0, writable, signer, name = "payer_acc", desc = "Fee payer account")]
        #[account(1, writable, name = "state_acc", desc = "State account")]
        CloseMapping,
    }
}
