use pinocchio::program_error::ProgramError;
use shank::ShankAccount;

use crate::state::DataLen;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, ShankAccount)]
pub struct MintMapping {
    pub mint: [u8; 32],
    pub price_chain: [u16; 4], // Conversion chain (e.g., [32, 0, u16::MAX, u16::MAX])
    pub decimals: u8,          // Mint decimals for price calculations
    pub is_active: bool,
    pub pyth_account: [u8; 33], // 0 = None, 1 = Some + 32 bytes
    pub switch_board: [u8; 33], // 0 = None, 1 = Some + 32 bytes
}

impl Default for MintMapping {
    fn default() -> Self {
        Self {
            mint: [0u8; 32],
            price_chain: [0u16; 4],
            decimals: 0,
            is_active: false,
            pyth_account: [0u8; 33],
            switch_board: [0u8; 33],
        }
    }
}

impl DataLen for MintMapping {
    const LEN: usize = core::mem::size_of::<MintMapping>();
}

impl MintMapping {
    pub fn set_pyth_account(&mut self, value: Option<[u8; 32]>) {
        match value {
            Some(val) => {
                self.pyth_account[0] = 1;
                self.pyth_account[1..].copy_from_slice(&val);
            }
            None => {
                self.pyth_account[0] = 0;
                self.pyth_account[1..].fill(0);
            }
        }
    }
    pub fn get_pyth_account(&self) -> Option<[u8; 32]> {
        if self.pyth_account[0] == 1 {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&self.pyth_account[1..]);
            Some(arr)
        } else {
            None
        }
    }
    pub fn set_switch_board(&mut self, value: Option<[u8; 32]>) {
        match value {
            Some(val) => {
                self.switch_board[0] = 1;
                self.switch_board[1..].copy_from_slice(&val);
            }
            None => {
                self.switch_board[0] = 0;
                self.switch_board[1..].fill(0);
            }
        }
    }
    pub fn get_switch_board(&self) -> Option<[u8; 32]> {
        if self.switch_board[0] == 1 {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&self.switch_board[1..]);
            Some(arr)
        } else {
            None
        }
    }

    /// Load a MintMapping from a byte array
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ProgramError> {
        if bytes.len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }

        // SAFETY: We've verified the byte length matches the struct size
        // and we're using #[repr(C)] which guarantees stable memory layout
        let mapping = unsafe { *(bytes.as_ptr() as *const Self) };
        Ok(mapping)
    }

    /// Convert a MintMapping to a byte array
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
}
