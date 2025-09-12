use pinocchio::{account_info::{AccountInfo, Ref, RefMut}, program_error::ProgramError, pubkey::Pubkey, *};
use shank::ShankAccount;

#[derive(Debug, Clone, Copy, PartialEq, ShankAccount)]
pub struct VestingRecordInfo {
    pub beneficiary: Pubkey,
    pub mint: Pubkey,
    pub total_vested_tokens: u64,
    pub total_claimed_tokens_by_beneficiary: u64,
    pub cliff_period: i64,
    pub vault_bump: u8,
    pub vesting_contract_info_bump: u8,
    pub bump: u8,
    pub beneficiary_ata: Pubkey,
    pub has_claimed: bool,
    pub start_time: i64,
    pub end_time: i64,
}

impl VestingRecordInfo {
    pub const SIZE: usize = core::mem::size_of::<VestingRecordInfo>();

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
            &mut *(data.as_ptr() as *mut Self)
        }))
    }
}