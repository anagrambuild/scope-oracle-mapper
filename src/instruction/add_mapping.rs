use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    sysvars::{rent::Rent, Sysvar},
    ProgramResult,
};
use pinocchio_system::instructions::Transfer;

use crate::{
    error::MappingProgramError,
    instruction::IntoBytes,
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

    if !payer_acc.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let account_size = core::alloc::Layout::from_size_align(
        state_acc.data_len() + MintMapping::LEN,
        core::mem::size_of::<u64>(),
    )
    .map_err(|_| ProgramError::InvalidAccountData)?
    .pad_to_align()
    .size();
    state_acc.realloc(account_size, false)?;
    let cost = Rent::get()?.minimum_balance(account_size);

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
    let mut registry = ScopeMappingRegistry::from_account_data(&acc_data)?;

    // CHECK if registry is initialized
    if !registry.is_initialized() {
        return Err(ProgramError::UninitializedAccount);
    }

    // Validate PDA
    ScopeMappingRegistry::validate_pda(registry.bump, state_acc.key(), payer_acc.key())?;

    if registry.owner.ne(payer_acc.key()) {
        return Err(MappingProgramError::InvalidOwner.into());
    }

    let ix_data = unsafe { load_ix_data::<AddMappingIxData>(data)? };

    let mapping = ix_data.mapping;
    // If you need to set pyth_account or switch_board, use the helpers:
    // mapping.set_pyth_account(...);
    // mapping.set_switch_board(...);

    registry.total_mappings += 1;
    registry.version += 1;

    let mapping_bytes = mapping.to_bytes();
    acc_data[ScopeMappingRegistry::LEN..ScopeMappingRegistry::LEN + MintMapping::LEN]
        .copy_from_slice(&mapping_bytes);

    Ok(())
}
