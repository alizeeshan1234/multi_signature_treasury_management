use pinocchio::{account_info::{AccountInfo, Ref, RefMut}, program_error::ProgramError, pubkey::Pubkey, *};
use shank::ShankAccount;

#[derive(Debug, Clone, Copy, PartialEq, ShankAccount)]
pub struct PaymentStreamingInfo {
    pub sender: Pubkey,
    pub recipient: Pubkey,
    pub sender_token_account: Pubkey,
    pub recipient_token_account: Pubkey,
    pub total_amount: u64,
    pub amount_per_hour: u64,
    // Employee hired on January 1st, 2024
    // Jan 1, 2024 00:00:00 UTC
    // Jan 31, 2024 23:59:59 UTC
    // Stream runs for exactly 31 days
    pub start_time: i64,
    pub end_time: i64,
    pub withdrawn_amount: u64,
    pub is_active: bool,
}

// stream instructions: 
// -> Initialize Stream (done)
// -> Withdraw From Stream (done)
// -> Cancel Stream  

impl PaymentStreamingInfo {
    pub const SIZE: usize = core::mem::size_of::<PaymentStreamingInfo>();

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