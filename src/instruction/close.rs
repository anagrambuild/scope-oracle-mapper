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
use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    sysvars::{rent::Rent, Sysvar},
    ProgramResult,
};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CloseMappingIxData {
    pub mint: [u8; 32],
    pub bump: u8,
}

impl DataLen for CloseMappingIxData {
    const LEN: usize = core::mem::size_of::<CloseMappingIxData>(); // 32 bytes for owner + 1 byte for bump
}

impl IntoBytes for CloseMappingIxData {
    fn into_bytes(&self) -> Result<&[u8], ProgramError> {
        Ok(unsafe { core::slice::from_raw_parts(self as *const Self as *const u8, Self::LEN) })
    }
}

pub fn process_close_mapping(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let [payer_acc, state_acc, _remaining @ ..] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !payer_acc.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if state_acc.data_is_empty() {
        return Err(ProgramError::UninitializedAccount);
    }

    // Validate that the account has enough data to be a valid registry
    if state_acc.data_len() < ScopeMappingRegistry::LEN {
        return Err(ProgramError::InvalidAccountData);
    }
    let ix_data = unsafe { load_ix_data::<CloseMappingIxData>(data)? };

    let mut cost_diff = 0;
    let mut new_size = 0;
    {
        // Validate that the account contains a valid registry
        let mut acc_data = state_acc.try_borrow_mut_data()?;
        let mut registry =
            ScopeMappingRegistry::from_slice(&acc_data[..ScopeMappingRegistry::LEN])?;
        if !registry.is_initialized() {
            return Err(ProgramError::UninitializedAccount);
        }

        // Hardcoded authority check
        if payer_acc.key().as_ref() != OWNER_PUB_KEY {
            return Err(MappingProgramError::InvalidOwner.into());
        }

        // Validate that the account is owned by our program
        if unsafe { state_acc.owner() } != &crate::ID {
            return Err(ProgramError::IncorrectProgramId);
        }

        // Validate the PDA
        ScopeMappingRegistry::validate_pda(ix_data.bump, state_acc.key(), payer_acc.key())?;

        // Find the mapping offsets using only the mutable borrow
        let (mint_mapping_offset, mint_mapping_end_offset) =
            MintMapping::get_mapping_details(&acc_data, &ix_data.mint)?;

        // remove the mapping by removing the data from the acc_data
        let remove_len = mint_mapping_end_offset - mint_mapping_offset;
        new_size = acc_data.len() - remove_len;
        let remaining_data = acc_data[mint_mapping_end_offset..].to_vec();

        acc_data[mint_mapping_offset..mint_mapping_offset + remaining_data.len()]
            .copy_from_slice(&remaining_data);

        // Zero out the trailing bytes to avoid data leakage
        for b in &mut acc_data[new_size..] {
            *b = 0;
        }
        cost_diff = Rent::get()?.minimum_balance(state_acc.data_len());

        registry.subtract_mapping(new_size as u16)?;

        let reg_bytes = registry.to_bytes();
        acc_data[..ScopeMappingRegistry::LEN].copy_from_slice(&reg_bytes);
    }

    state_acc.realloc(new_size, false)?;

    if cost_diff > 0 {
        unsafe {
            *state_acc.borrow_mut_lamports_unchecked() = state_acc.lamports() - cost_diff;
            *payer_acc.borrow_mut_lamports_unchecked() = payer_acc.lamports() + cost_diff;
        };
    }

    Ok(())
}
