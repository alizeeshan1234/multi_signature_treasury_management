use pinocchio::{account_info::{AccountInfo, Ref, RefMut}, program_error::ProgramError, pubkey::Pubkey, ProgramResult};
use shank::ShankAccount;

#[derive(Debug, Clone, Copy, PartialEq, ShankAccount)]
pub struct StreamProposal {
    pub proposer: Pubkey,
    pub stream_name: [u8; 32],           
    pub stream_description: [u8; 128],   
    pub proposal_id: u64,
    pub multisig_id: u64,
    pub stream_type: u8,
    pub created_at: i64,
    pub voting_deadline: i64,
    pub approvals: [Pubkey; 10],         
    pub approval_count: u8,
    pub rejections: [Pubkey; 10],        
    pub rejection_count: u8,
    pub total_vote_count: u64,
    pub required_threshold: u8,
    pub status: u8,
}

impl StreamProposal {
    pub const SIZE: usize = core::mem::size_of::<StreamProposal>();

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

#[derive(Clone, Debug, PartialEq)]
pub enum StreamType {
    BatchPayments,
    TokenSwaps,
    TokenTransfers,
    PaymentStreaming,
    Vesting,
    OneTimePayment,
}

impl TryFrom<&u8> for StreamType {
    type Error = ProgramError;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match *value {
            0 => Ok(StreamType::BatchPayments),
            1 => Ok(StreamType::TokenSwaps),
            2 => Ok(StreamType::TokenTransfers),
            3 => Ok(StreamType::PaymentStreaming),
            4 => Ok(StreamType::Vesting),
            5 => Ok(StreamType::OneTimePayment),
            _ => Err(ProgramError::InvalidAccountData)
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ProposalStatus {
    Draft,           
    Pending,          
    UnderReview,     
    Approved,        
    Executed,        
    Active,          
    Completed,       
    Rejected,        
    Expired,         
    Failed,          
    Cancelled,       
    Revoked,         
    Paused,          
    Disputed,        
    RequiresUpdate,  
}

impl TryFrom<&u8> for ProposalStatus {
    type Error = ProgramError;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match *value {
            0 => Ok(ProposalStatus::Draft),
            1 => Ok(ProposalStatus::Pending),
            2 => Ok(ProposalStatus::UnderReview),
            3 => Ok(ProposalStatus::Approved),
            4 => Ok(ProposalStatus::Executed),
            5 => Ok(ProposalStatus::Active),
            6 => Ok(ProposalStatus::Completed),
            7 => Ok(ProposalStatus::Rejected),
            8 => Ok(ProposalStatus::Expired),
            9 => Ok(ProposalStatus::Failed),
            10 => Ok(ProposalStatus::Cancelled),
            11 => Ok(ProposalStatus::Revoked),
            12 => Ok(ProposalStatus::Paused),
            13 => Ok(ProposalStatus::Disputed),
            14 => Ok(ProposalStatus::RequiresUpdate),
            _ => Err(ProgramError::InvalidAccountData)
        }
    }
}