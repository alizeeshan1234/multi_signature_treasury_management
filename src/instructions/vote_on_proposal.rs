use pinocchio::{account_info::AccountInfo, instruction::Signer, program_error::ProgramError, pubkey::Pubkey, sysvars::{clock::Clock, rent::Rent, Sysvar}, *};

use crate::states::{MultiSignatureVault, StreamProposal, ProposalStatus};

pub fn process_vote_on_proposal(accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {

    let [voter, stream_proposal_account, multisig_account, _system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !voter.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if instruction_data.len() < 24 {
        return Err(ProgramError::InvalidInstructionData);
    }

    let proposal_id = u64::from_le_bytes(
        instruction_data[0..8].try_into().map_err(|_| ProgramError::InvalidInstructionData)?
    );

    let multisig_id = u64::from_le_bytes(
        instruction_data[8..16].try_into().map_err(|_| ProgramError::InvalidInstructionData)?
    );

    let vote_type = u64::from_le_bytes(
        instruction_data[16..24].try_into().map_err(|_| ProgramError::InvalidInstructionData)?
    );

    if vote_type > 1 {
        return Err(ProgramError::InvalidInstructionData);
    }

    let (stream_proposal_account_pda, _bump) = pubkey::find_program_address(
        &[b"stream_proposal", proposal_id.to_le_bytes().as_ref(), multisig_id.to_le_bytes().as_ref()],
        &crate::ID
    );

    if *stream_proposal_account.key() != stream_proposal_account_pda {
        return Err(ProgramError::InvalidAccountData);
    }

    let mut stream_proposal_account_info = StreamProposal::from_account_info_mut(stream_proposal_account)?;

    if stream_proposal_account_info.multisig_id != multisig_id {
        return Err(ProgramError::InvalidAccountData);
    }

    let current_time = Clock::get()?.unix_timestamp;

    if current_time > stream_proposal_account_info.voting_deadline {
        return Err(ProgramError::Custom(2001)); 
    }

    let proposal_status = ProposalStatus::try_from(&stream_proposal_account_info.status)?;
    if proposal_status != ProposalStatus::Active {
        return Err(ProgramError::Custom(2003)); 
    }

    let multisig_account_info = MultiSignatureVault::from_account_info(multisig_account)?;

    let (multisig_info_pda, _multisig_info_bump) = pubkey::find_program_address(
        &[b"multisig_info", multisig_account_info.admin.as_ref(), multisig_id.to_le_bytes().as_ref()],
        &crate::ID
    );

    if *multisig_account.key() != multisig_info_pda {
        return Err(ProgramError::InvalidAccountData);
    }

    if !multisig_account_info.is_active {
        return Err(ProgramError::Custom(2004)); 
    }

    let voter_key = *voter.key();
    let mut is_multisig_member = false;
    
    for member in multisig_account_info.member_keys {
        if voter_key == member {
            is_multisig_member = true;
            break;
        }
    }

    if !is_multisig_member {
        return Err(ProgramError::Custom(2005)); 
    }

    for approval in stream_proposal_account_info.approvals {
        if approval == voter_key {
            return Err(ProgramError::Custom(2002)); 
        }
    }
    
    for rejection in stream_proposal_account_info.rejections {
        if rejection == voter_key {
            return Err(ProgramError::Custom(2002)); 
        }
    }

    match vote_type {
        0 => { 
            let mut vote_recorded = false;
            
            for i in 0..10 {
                if stream_proposal_account_info.approvals[i] == Pubkey::default() {
                    stream_proposal_account_info.approvals[i] = voter_key;
                    vote_recorded = true;
                    
                    stream_proposal_account_info.approval_count = 
                        stream_proposal_account_info.approval_count
                        .checked_add(1)
                        .ok_or(ProgramError::ArithmeticOverflow)?;
                    
                    stream_proposal_account_info.total_vote_count = 
                        stream_proposal_account_info.total_vote_count
                        .checked_add(1)
                        .ok_or(ProgramError::ArithmeticOverflow)?;
                    
                    break;
                }
            }

            if !vote_recorded {
                return Err(ProgramError::InvalidAccountData); 
            }
        },
        1 => { 
            let mut vote_recorded = false;
            
            for i in 0..10 {
                if stream_proposal_account_info.rejections[i] == Pubkey::default() {
                    stream_proposal_account_info.rejections[i] = voter_key;
                    vote_recorded = true;
                    
                    stream_proposal_account_info.rejection_count = 
                        stream_proposal_account_info.rejection_count
                        .checked_add(1)
                        .ok_or(ProgramError::ArithmeticOverflow)?;
                    
                    stream_proposal_account_info.total_vote_count = 
                        stream_proposal_account_info.total_vote_count
                        .checked_add(1)
                        .ok_or(ProgramError::ArithmeticOverflow)?;
                    
                    break;
                }
            }

            if !vote_recorded {
                return Err(ProgramError::InvalidAccountData); 
            }
        },
        _ => {
            return Err(ProgramError::InvalidInstructionData);
        }
    }

    if stream_proposal_account_info.rejection_count >= stream_proposal_account_info.required_threshold {
        stream_proposal_account_info.status = ProposalStatus::Rejected as u8;
    }

    else if stream_proposal_account_info.approval_count >= stream_proposal_account_info.required_threshold {
        stream_proposal_account_info.status = ProposalStatus::Approved as u8;
    }

    Ok(())
}