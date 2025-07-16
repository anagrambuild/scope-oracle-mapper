#![allow(unexpected_cfgs)]

use crate::instruction::{self, InstructionSet};
use pinocchio::{
    account_info::AccountInfo, default_panic_handler, msg, program_entrypoint,
    program_error::ProgramError, pubkey::Pubkey, ProgramResult,
};

// This is the entrypoint for the program.
program_entrypoint!(process_instruction);
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
        InstructionSet::InitializeState => {
            msg!("Ix:0 -> InitializeState");
            instruction::process_initialize_state(accounts, instruction_data)
        }
        InstructionSet::AddMapping => {
            msg!("Ix:1 -> AddMapping");
            instruction::process_add_mapping(accounts, instruction_data)
        }
    }
}
