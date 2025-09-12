use pinocchio::{account_info::{AccountInfo, Ref, RefMut}, program_error::ProgramError, pubkey::Pubkey, *};
use shank::ShankAccount;

#[derive(Debug, Clone, Copy, PartialEq, ShankAccount)]
pub struct VestingContractInfo {
    pub creator: Pubkey,
    pub company_name: [u8; 32],
    pub mint: Pubkey,
    pub total_vested_tokens: u64, //The number of tokens that the company has deposited in the vault
    pub total_available_tokens: u64, //The number of tokens that are currently available for distribution
    //Increases when the company initializes a new beneficiary
    //Company also specifies the number of tokens that the beneficiary will receive ex : 1000 tokens
    pub total_locked_tokens: u64, // Total tokens locked in vesting contracts(Total Vested Tokens)
    pub total_claimed_tokens: u64, 
    pub vault_account: Pubkey,
    pub created_at: i64,
    pub vault_bump: u8,
    pub bump: u8,
    pub is_active: bool,
    pub fully_claimed: bool,
}

impl VestingContractInfo {
    pub const SIZE: usize = core::mem::size_of::<VestingContractInfo>();

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