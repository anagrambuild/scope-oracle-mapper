extern crate alloc;

use super::utils::{DataLen, Initialized};
use alloc::vec::Vec;
use pinocchio::{
    program_error::ProgramError,
    pubkey::{self, Pubkey},
    ProgramResult,
};

use crate::error::MappingProgramError;
use crate::state::mint_mapping::MintMapping;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, shank::ShankAccount)]
pub struct ScopeMappingRegistry {
    pub is_initialized: u8,
    pub owner: [u8; 32],
    pub total_mappings: u32,
    pub version: u8,
    pub bump: u8,
    // Remove the fixed array and we'll handle mappings separately
}

impl DataLen for ScopeMappingRegistry {
    const LEN: usize = core::mem::size_of::<ScopeMappingRegistry>();
}

impl Initialized for ScopeMappingRegistry {
    fn is_initialized(&self) -> bool {
        self.is_initialized > 0
    }
}

impl ScopeMappingRegistry {
    pub const SEED: &'static str = "ScopeMappingRegistry";

    pub fn validate_pda(bump: u8, pda: &Pubkey, owner: &Pubkey) -> Result<(), ProgramError> {
        let seed_with_bump = &[Self::SEED.as_bytes(), owner, &[bump]];
        let derived = pubkey::create_program_address(seed_with_bump, &crate::ID)?;
        pinocchio::msg!(
            "[DEBUG] validate_pda: bump={:?}, pda={:?}, owner={:?}, derived={:?}, program_id={:?}",
            bump,
            pda,
            owner,
            derived,
            crate::ID
        );
        if derived != *pda {
            return Err(MappingProgramError::PdaMismatch.into());
        }
        Ok(())
    }

    pub fn add(&mut self) -> ProgramResult {
        if !self.is_initialized() {
            return Err(ProgramError::InvalidAccountData);
        }
        self.total_mappings += 1;
        self.version += 1;
        Ok(())
    }

    /// Load a ScopeMappingRegistry from a byte array
    pub fn from_bytes(bytes: &[u8; Self::LEN]) -> Result<Self, ProgramError> {
        if bytes.len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }

        // SAFETY: We've verified the byte length matches the struct size
        // and we're using #[repr(C)] which guarantees stable memory layout
        let mapping = unsafe { *(bytes.as_ptr() as *const Self) };
        Ok(mapping)
    }

    /// Convert a ScopeMappingRegistry to a byte array
    pub fn to_bytes(&self) -> [u8; Self::LEN] {
        let mut bytes = [0u8; Self::LEN];

        // SAFETY: We're using #[repr(C)] which guarantees stable memory layout
        unsafe {
            core::ptr::copy_nonoverlapping(
                self as *const Self as *const u8,
                bytes.as_mut_ptr(),
                Self::LEN,
            );
        }
        bytes
    }

    /// Load a ScopeMappingRegistry from a slice of bytes
    pub fn from_slice(bytes: &[u8]) -> Result<Self, ProgramError> {
        if bytes.len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }

        // SAFETY: We've verified the byte length matches the struct size
        let mapping = unsafe { *(bytes.as_ptr() as *const Self) };
        Ok(mapping)
    }

    /// Convert a ScopeMappingRegistry to a byte vector
    pub fn to_vec(&self) -> Vec<u8> {
        let bytes = self.to_bytes();
        bytes.to_vec()
    }

    /// Given the full account data, split into registry and mappings vector
    pub fn from_account_data(data: &[u8]) -> Result<Self, ProgramError> {
        if data.len() < Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        let registry = Self::from_slice(&data[..Self::LEN])?;
        Ok(registry)
    }

    /// Write the registry and mappings vector to the account data
    pub fn to_account_data(
        registry: &Self,
        mapping: &MintMapping,
        data: &mut [u8],
    ) -> Result<(), ProgramError> {
        let reg_bytes = registry.to_bytes();
        data[..Self::LEN].copy_from_slice(&reg_bytes);
        let mapping_bytes = mapping.to_bytes();
        data[Self::LEN..Self::LEN + MintMapping::LEN].copy_from_slice(&mapping_bytes);
        Ok(())
    }

    /// Get the mappings slice from the account data
    pub fn get_mappings_slice(data: &[u8]) -> Result<&[u8], ProgramError> {
        if data.len() < Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(&data[Self::LEN..])
    }
}
