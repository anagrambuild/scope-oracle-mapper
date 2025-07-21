#![allow(unexpected_cfgs)]

use crate::instruction::{
    process_add_mapping, process_close_mapping, process_initialize_state, InstructionSet,
};
use pinocchio::{
    account_info::AccountInfo, default_panic_handler, no_allocator, program_entrypoint,
    program_error::ProgramError, pubkey::Pubkey, ProgramResult,
};

// This is the entrypoint for the program.
program_entrypoint!(process_instruction);
//Do not allocate memory.
no_allocator!();
// Use the no_std panic handler.
default_panic_handler!();

#[inline(always)]
fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let (ix_disc, instruction_data) = instruction_data
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;

    match InstructionSet::try_from(ix_disc)? {
        InstructionSet::InitializeState => process_initialize_state(accounts, instruction_data),
        InstructionSet::AddMapping => process_add_mapping(accounts, instruction_data),
        InstructionSet::CloseMapping => process_close_mapping(accounts, instruction_data),
    }
}
