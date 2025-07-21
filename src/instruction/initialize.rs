use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    sysvars::rent::Rent,
    ProgramResult,
};
use pinocchio_system::instructions::CreateAccount;

use crate::{
    error::MappingProgramError,
    instruction::{IntoBytes, OWNER_PUB_KEY},
    state::{
        scope_mapping_registry::ScopeMappingRegistry,
        try_from_account_info_mut,
        utils::{load_ix_data, DataLen},
    },
};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, shank::ShankType)]
pub struct InitializeRegistryIxData {
    pub owner: [u8; 32],
    pub bump: u8,
}

impl DataLen for InitializeRegistryIxData {
    const LEN: usize = core::mem::size_of::<InitializeRegistryIxData>(); // 32 bytes for owner + 1 byte for bump
}

impl IntoBytes for InitializeRegistryIxData {
    fn into_bytes(&self) -> Result<&[u8], ProgramError> {
        Ok(unsafe { core::slice::from_raw_parts(self as *const Self as *const u8, Self::LEN) })
    }
}

pub fn process_initialize_state(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let [payer_acc, state_acc, sysvar_rent_acc, ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !payer_acc.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if !state_acc.data_is_empty() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    let rent = Rent::from_account_info(sysvar_rent_acc)?;

    let ix_data = unsafe { load_ix_data::<InitializeRegistryIxData>(data)? };

    // Hardcoded authority check
    if payer_acc.key().as_ref() != OWNER_PUB_KEY || ix_data.owner != OWNER_PUB_KEY {
        return Err(MappingProgramError::InvalidOwner.into());
    }

    let pda_bump_bytes = [ix_data.bump];

    // Validate the PDA
    ScopeMappingRegistry::validate_pda(ix_data.bump, state_acc.key(), payer_acc.key())?;

    // Signer seeds
    let signer_seeds = [
        Seed::from(ScopeMappingRegistry::SEED.as_bytes()),
        Seed::from(&ix_data.owner),
        Seed::from(&pda_bump_bytes[..]),
    ];
    let signers = [Signer::from(&signer_seeds[..])];

    // Create the account
    CreateAccount {
        from: payer_acc,
        to: state_acc,
        space: ScopeMappingRegistry::LEN as u64,
        owner: &crate::ID,
        lamports: rent.minimum_balance(ScopeMappingRegistry::LEN),
    }
    .invoke_signed(&signers)?;

    // Initialize the account data using the proper method
    let scope_reg_data = unsafe { try_from_account_info_mut::<ScopeMappingRegistry>(state_acc) }?;

    scope_reg_data.owner = ix_data.owner;
    scope_reg_data.total_mappings = 0;
    scope_reg_data.version = 0;
    scope_reg_data.bump = ix_data.bump;
    scope_reg_data.is_initialized = 1;

    Ok(())
}
