use pinocchio::program_error::ProgramError;

#[derive(Clone, PartialEq, shank::ShankType)]
pub enum MappingProgramError {
    // overflow error
    WriteOverflow,
    // invalid instruction data
    InvalidInstructionData,
    // pda mismatch
    PdaMismatch,
    // Invalid Owner
    InvalidOwner,
    // Max Mappings Reached
    MaxMappingsReached,
    // Invalid Mapping Size
    InvalidMappingSize,
    // Mint Not Found
    MintNotFound,
    // Invalid Account Data
    InvalidAccountData,
}

impl From<MappingProgramError> for ProgramError {
    fn from(e: MappingProgramError) -> Self {
        Self::Custom(e as u32)
    }
}
