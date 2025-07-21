use pinocchio::program_error::ProgramError;
use shank::ShankAccount;

use crate::{
    error::MappingProgramError,
    state::{DataLen, ScopeMappingRegistry},
};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, ShankAccount, Default)]
pub struct MintMapping {
    // Mapping details
    // 000 -> None/ disables,
    // 001 -> scope,
    // 010 -> pyth,
    // 011 -> pyth + scope,
    // 100 -> switch_board,
    // 101 -> switch_board + scope,
    // 110 -> switch_board + pyth,
    // 111 -> switch_board + pyth + scope,
    pub mint: [u8; 32],
    pub offset: u8, // Offset for the mapping
    pub mapping_details: u8,
    pub decimals: u8,                    // Mint decimals for price calculations
    pub scope_details: Option<[u16; 3]>, // Conversion chain (e.g., [32, 0, u16::MAX, u16::MAX])
    pub pyth_account: Option<[u8; 32]>,  // 0 = None, 1 = Some + 32 bytes
    pub switch_board: Option<[u8; 32]>,  // 0 = None, 1 = Some + 32 bytes
}

impl MintMapping {
    pub fn set_pyth_account(&mut self, value: Option<[u8; 32]>) {
        self.pyth_account = value;
    }

    pub fn get_pyth_account(&self) -> Option<[u8; 32]> {
        self.pyth_account
    }
    pub fn set_switch_board(&mut self, value: Option<[u8; 32]>) {
        self.switch_board = value;
    }
    pub fn get_switch_board(&self) -> Option<[u8; 32]> {
        self.switch_board
    }

    pub fn new(
        mint: [u8; 32],
        scope_details: Option<[u16; 3]>,
        pyth_account: Option<[u8; 32]>,
        switch_board: Option<[u8; 32]>,
        decimals: u8,
    ) -> Self {
        let mut mapping = Self::default();
        let mut mapping_details = 0;
        if scope_details.is_some() {
            mapping_details |= 0b001;
        }
        if pyth_account.is_some() {
            mapping_details |= 0b010;
        }
        if switch_board.is_some() {
            mapping_details |= 0b100;
        }

        mapping.mint = mint;
        mapping.mapping_details = mapping_details;
        mapping.decimals = decimals;
        mapping.scope_details = scope_details;
        mapping.pyth_account = pyth_account;
        mapping.switch_board = switch_board;
        mapping.offset = mapping.serialized_size() as u8;
        mapping
    }

    /// Load a MintMapping from a byte array with extreme efficiency
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ProgramError> {
        if bytes.len() < 35 {
            return Err(ProgramError::InvalidAccountData);
        }

        let mut mapping = Self::default();
        mapping.mint.copy_from_slice(&bytes[0..32]);
        mapping.offset = bytes[32];
        mapping.mapping_details = bytes[33];
        mapping.decimals = bytes[34];

        let mut data_offset = 35;

        // Ultra-fast deserialization using bit manipulation
        if (mapping.mapping_details & 0b001) != 0 && data_offset + 6 <= bytes.len() {
            let mut scope = [0u16; 3];
            scope[0] = u16::from_le_bytes([bytes[data_offset], bytes[data_offset + 1]]);
            scope[1] = u16::from_le_bytes([bytes[data_offset + 2], bytes[data_offset + 3]]);
            scope[2] = u16::from_le_bytes([bytes[data_offset + 4], bytes[data_offset + 5]]);
            mapping.scope_details = Some(scope);
            data_offset += 6;
        }

        if (mapping.mapping_details & 0b010) != 0 && data_offset + 32 <= bytes.len() {
            let mut pyth = [0u8; 32];
            pyth.copy_from_slice(&bytes[data_offset..data_offset + 32]);
            mapping.pyth_account = Some(pyth);
            data_offset += 32;
        }

        if (mapping.mapping_details & 0b100) != 0 && data_offset + 32 <= bytes.len() {
            let mut switch = [0u8; 32];
            switch.copy_from_slice(&bytes[data_offset..data_offset + 32]);
            mapping.switch_board = Some(switch);
        }

        Ok(mapping)
    }

    /// Convert a MintMapping to a byte array with extreme efficiency
    /// Returns exact size needed (35-71 bytes) - includes offset field
    pub fn to_bytes(&self) -> [u8; 105] {
        let mut bytes = [0; 105];

        // Header: mint(32) + offset(1) + mapping_details(1) + decimals(1) = 35 bytes
        bytes[0..32].copy_from_slice(&self.mint);
        // Offset for the mapping written in the last at index 32
        bytes[33] = self.mapping_details;
        bytes[34] = self.decimals;

        let mut data_offset = 35;

        // Bit 0: scope_details (6 bytes)
        if (self.mapping_details & 0b001) != 0 && self.scope_details.is_some() {
            let scope = self.scope_details.unwrap();
            bytes[data_offset..data_offset + 2].copy_from_slice(&scope[0].to_le_bytes());
            bytes[data_offset + 2..data_offset + 4].copy_from_slice(&scope[1].to_le_bytes());
            bytes[data_offset + 4..data_offset + 6].copy_from_slice(&scope[2].to_le_bytes());
            data_offset += 6;
        }

        // Bit 1: pyth_account (32 bytes)
        if (self.mapping_details & 0b010) != 0 && self.pyth_account.is_some() {
            bytes[data_offset..data_offset + 32].copy_from_slice(&self.pyth_account.unwrap());
            data_offset += 32;
        }

        // Bit 2: switch_board (32 bytes)
        if (self.mapping_details & 0b100) != 0 && self.switch_board.is_some() {
            bytes[data_offset..data_offset + 32].copy_from_slice(&self.switch_board.unwrap());
            data_offset += 32;
        }

        bytes[32] = data_offset as u8;

        bytes
    }

    pub fn serialized_size(&self) -> u16 {
        let mut size = 35; // mint(32) + offset(1) + mapping_details(1) + decimals(1)

        if (self.mapping_details & 0b001) != 0 && self.scope_details.is_some() {
            size += 6;
        }
        if (self.mapping_details & 0b010) != 0 && self.pyth_account.is_some() {
            size += 32;
        }
        if (self.mapping_details & 0b100) != 0 && self.switch_board.is_some() {
            size += 32;
        }

        size as u16
    }

    pub fn is_valid(&self) -> bool {
        let has_scope = (self.mapping_details & 0b001) != 0;
        let has_pyth = (self.mapping_details & 0b010) != 0;
        let has_switch = (self.mapping_details & 0b100) != 0;

        (has_scope == self.scope_details.is_some())
            && (has_pyth == self.pyth_account.is_some())
            && (has_switch == self.switch_board.is_some())
    }

    pub fn has_scope(&self) -> bool {
        (self.mapping_details & 0b001) != 0
    }
    pub fn has_pyth(&self) -> bool {
        (self.mapping_details & 0b010) != 0
    }
    pub fn has_switch_board(&self) -> bool {
        (self.mapping_details & 0b100) != 0
    }

    /// Set mapping details with validation
    pub fn set_mapping_details(&mut self, details: u8) {
        self.mapping_details = details & 0b111; // Ensure only 3 bits are used
    }

    pub fn enabled_components(&self) -> u8 {
        self.mapping_details.count_ones() as u8
    }

    pub fn get_mapping_details(
        data: &[u8],
        mint: &[u8; 32],
    ) -> Result<(usize, usize), MappingProgramError> {
        let mut total_size = unsafe { *data.get_unchecked(34) } as u16;
        let mut offset = ScopeMappingRegistry::LEN;

        while total_size > 0 {
            let mint_end_offset = unsafe { *data.get_unchecked(offset + 32) } as usize;
            if data[offset..offset + 32] == *mint {
                return Ok((offset, offset + mint_end_offset));
            }
            offset += mint_end_offset;
            total_size -= 1;
        }

        Err(MappingProgramError::MintNotFound)
    }
}
