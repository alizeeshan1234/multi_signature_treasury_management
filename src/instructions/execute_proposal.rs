use pinocchio::{account_info::AccountInfo, instruction::Signer, program_error::ProgramError, pubkey::Pubkey, sysvars::{clock::Clock, Sysvar}, *};
use crate::{instructions, states::{MultiSignatureVault, ProposalStatus, StreamProposal, StreamType}};

pub fn process_execute_proposal(accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {

    let [executor, stream_proposal_account, multisig_account] = accounts else {
        return Err(ProgramError::InvalidAccountData);
    };

    if !executor.is_signer() {
        return Err(ProgramError::InvalidAccountData);
    };

    let mut stream_proposal = StreamProposal::from_account_info_mut(stream_proposal_account)?;
    let multisig_info = MultiSignatureVault::from_account_info(multisig_account)?;
    
    if !can_execute_proposal(&stream_proposal, &multisig_info)? {
        msg!("Proposal cannot be executed - threshold not met or expired");
        return Err(ProgramError::InvalidAccountData);
    };

    verify_multisig_execution(&stream_proposal, &multisig_info, executor.key())?;

    let stream_type = StreamType::try_from(&stream_proposal.stream_type)?;

    match stream_type {
        StreamType::BatchPayments => {
            instructions::init_batch_payments::process_initialize_batch_state(accounts, instruction_data)?;
            instructions::batch_payments::process_batch_transfer(accounts, instruction_data)?;

            stream_proposal.status = ProposalStatus::Executed as u8;
        },
        StreamType::TokenTransfers => {
            instructions::token_transfers::process_transfer_tokens(accounts, instruction_data)?;
            
            stream_proposal.status = ProposalStatus::Executed as u8;
        },
        StreamType::PaymentStreaming => {
            instructions::init_stream_payments::process_init_stream_payment(accounts, instruction_data)?;

            stream_proposal.status = ProposalStatus::Active as u8;
        },
        StreamType::Vesting => {
            instructions::init_vesting::process_init_vesting_contract(accounts, instruction_data)?;
            instructions::init_beneficiary::process_init_beneficiary(accounts, instruction_data)?;

            stream_proposal.status = ProposalStatus::Active as u8;
        }
    }

    msg!("Stream proposal executed successfully!");

    Ok(())
}

fn can_execute_proposal(
    proposal: &StreamProposal, 
    multisig_info: &MultiSignatureVault
) -> Result<bool, ProgramError> {
    let current_time = Clock::get()?.unix_timestamp;
    
    if current_time > proposal.voting_deadline {
        return Ok(false);
    }

    if !multisig_info.is_active {
        return Ok(false);
    }

    let required_threshold = multisig_info.threshold as u8;
    
    if proposal.approval_count < required_threshold {
        return Ok(false);
    }

    if proposal.approval_count <= proposal.rejection_count {
        return Ok(false);
    }

    let proposal_status = ProposalStatus::try_from(&proposal.status)?;
    if proposal_status != ProposalStatus::Approved {
        return Ok(false);
    }
    
    Ok(true)
}

fn verify_multisig_execution(
    proposal: &StreamProposal,
    multisig_info: &MultiSignatureVault,
    executor_key: &Pubkey,
) -> ProgramResult {
    let mut is_multisig_member = false;
    for member in multisig_info.member_keys {
        if member == *executor_key {
            is_multisig_member = true;
            break;
        }
    }

    if !is_multisig_member {
        msg!("Executor is not a multisig member");
        return Err(ProgramError::Custom(2005)); 
    }

    let mut executor_approved = false;
    for i in 0..proposal.approval_count as usize {
        if proposal.approvals[i] == *executor_key {
            executor_approved = true;
            break;
        }
    }

    if !executor_approved {
        msg!("Executor did not approve this proposal");
        return Err(ProgramError::InvalidAccountData);
    }

    if proposal.multisig_id != multisig_info.id {
        msg!("Proposal multisig_id mismatch");
        return Err(ProgramError::InvalidAccountData);
    }

    Ok(())
}