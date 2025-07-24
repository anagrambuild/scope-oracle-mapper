use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    sysvars::{rent::Rent, Sysvar},
    ProgramResult,
};
use pinocchio_system::instructions::Transfer;

use crate::{
    error::MappingProgramError,
    instruction::{IntoBytes, OWNER_PUB_KEY},
    state::{
        mint_mapping::MintMapping,
        scope_mapping_registry::ScopeMappingRegistry,
        utils::{load_ix_data, DataLen},
        Initialized,
    },
};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AddMappingIxData {
    pub mapping: MintMapping,
}

impl DataLen for AddMappingIxData {
    const LEN: usize = core::mem::size_of::<AddMappingIxData>(); // 32 bytes for data
}

impl IntoBytes for AddMappingIxData {
    fn into_bytes(&self) -> Result<&[u8], ProgramError> {
        Ok(unsafe { core::slice::from_raw_parts(self as *const Self as *const u8, Self::LEN) })
    }
}

pub fn process_add_mapping(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let [payer_acc, state_acc, ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Hardcoded authority check
    if payer_acc.key().as_ref() != OWNER_PUB_KEY {
        return Err(MappingProgramError::InvalidOwner.into());
    }

    if !payer_acc.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }
    let end_offset = state_acc.data_len();

    let ix_data = unsafe { load_ix_data::<AddMappingIxData>(data)? };

    let mapping = ix_data.mapping;
    let mapping_size = mapping.serialized_size();

    let new_account_size = end_offset + mapping_size as usize;

    state_acc.resize(new_account_size)?;
    let cost = Rent::get()?.minimum_balance(new_account_size);
    if cost > 0 {
        Transfer {
            from: payer_acc,
            to: state_acc,
            lamports: cost - state_acc.lamports(),
        }
        .invoke()?;
    }

    // Get the full account data as a mutable slice
    let mut acc_data = state_acc.try_borrow_mut_data()?;
    let mut registry = unsafe { *(acc_data.as_ptr() as *const ScopeMappingRegistry) };

    // CHECK if registry is initialized
    if !registry.is_initialized() {
        return Err(ProgramError::UninitializedAccount);
    }

    // Validate PDA
    ScopeMappingRegistry::validate_pda(registry.bump, state_acc.key(), payer_acc.key())?;

    if registry.owner.ne(payer_acc.key()) {
        return Err(MappingProgramError::InvalidOwner.into());
    }

    let old_last_mapping_offset = registry.last_mapping_offset + ScopeMappingRegistry::LEN as u16;
    registry.add(mapping_size)?;

    // Write the updated registry back to account data
    let reg_bytes = registry.to_bytes();
    acc_data[..ScopeMappingRegistry::LEN].copy_from_slice(&reg_bytes);

    let mapping_bytes = mapping.to_bytes();
    acc_data[old_last_mapping_offset as usize..(old_last_mapping_offset + mapping_size) as usize]
        .copy_from_slice(&mapping_bytes[..mapping_size as usize]);

    Ok(())
}
