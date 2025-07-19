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

pub const MAX_MAPPINGS: u32 = 512;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, shank::ShankAccount)]
pub struct ScopeMappingRegistry {
    pub is_initialized: u8,
    pub owner: [u8; 32],
    pub total_mappings: u32,
    pub version: u8,
    pub bump: u8,
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
        if derived != *pda {
            return Err(MappingProgramError::PdaMismatch.into());
        }
        Ok(())
    }

    pub fn add(&mut self) -> ProgramResult {
        if !self.is_initialized() {
            return Err(ProgramError::InvalidAccountData);
        }
        if self.total_mappings >= MAX_MAPPINGS as u32 {
            return Err(MappingProgramError::MaxMappingsReached.into());
        }
        self.total_mappings += 1;
        self.version += 1;
        Ok(())
    }

    pub fn from_bytes(bytes: &[u8; Self::LEN]) -> Result<Self, ProgramError> {
        if bytes.len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }

        let mapping = unsafe { *(bytes.as_ptr() as *const Self) };
        Ok(mapping)
    }

    pub fn to_bytes(&self) -> [u8; Self::LEN] {
        let mut bytes = [0u8; Self::LEN];

        unsafe {
            core::ptr::copy_nonoverlapping(
                self as *const Self as *const u8,
                bytes.as_mut_ptr(),
                Self::LEN,
            );
        }
        bytes
    }

    pub fn from_slice(bytes: &[u8]) -> Result<Self, ProgramError> {
        if bytes.len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }

        let mapping = unsafe { *(bytes.as_ptr() as *const Self) };
        Ok(mapping)
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let bytes = self.to_bytes();
        bytes.to_vec()
    }

    pub fn from_account_data(data: &[u8]) -> Result<Self, ProgramError> {
        if data.len() < Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        let registry = Self::from_slice(&data[..Self::LEN])?;
        Ok(registry)
    }

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

    pub fn get_mappings_slice(data: &[u8]) -> Result<&[u8], ProgramError> {
        if data.len() < Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(&data[Self::LEN..])
    }
}
