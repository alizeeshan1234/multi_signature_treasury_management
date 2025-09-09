use pinocchio::{account_info::{AccountInfo, Ref, RefMut}, program_error::ProgramError, pubkey::Pubkey, *};
use shank::ShankAccount;

#[derive(Debug, Clone, Copy, PartialEq, ShankAccount)]
pub struct MultiSignatureVault {
    pub id: u64,
    pub admin: Pubkey,
    pub is_active: bool,              // Vault status
    pub member_count: u64,
    pub member_keys: [Pubkey; 10],
    pub threshold: u64,
    pub proposal_expiry: i64, 
    pub total_proposals: u64,
    pub treasury_vault: Pubkey, 
    pub created_at: i64,              // Timestamp
    pub last_updated: i64,            // Last modification
    pub minimum_balance: u64,         // Treasury threshold
    pub active_proposals: u64,        // Current pending proposals
    pub executed_proposals: u64,      // Successfully executed count
    pub bump: u8,
    pub treasury_vault_bump: u8,
}

impl MultiSignatureVault {
    pub const SIZE: usize = core::mem::size_of::<MultiSignatureVault>();

    pub fn from_account_info(account: &AccountInfo) -> Result<Ref<Self>, ProgramError> {
        if account.data_len() < Self::SIZE {
            return Err(ProgramError::InvalidAccountData);
        };

        Ok(Ref::map(account.try_borrow_data()?, |data| unsafe {
            &*(data.as_ptr() as *const Self)
        }))
    }

    pub fn from_account_info_mut(account: &AccountInfo) -> Result<RefMut<Self>, ProgramError> {
        if account.data_len() < Self::SIZE {
            return Err(ProgramError::InvalidAccountData);
        };

        Ok(RefMut::map(account.try_borrow_mut_data()?, |data| unsafe {
            &mut *(data.as_mut_ptr() as *mut Self)
        }))
    }
}