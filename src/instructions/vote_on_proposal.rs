use pinocchio::{account_info::AccountInfo, instruction::Signer, program_error::ProgramError, pubkey::Pubkey, sysvars::{clock::Clock, rent::Rent, Sysvar}, *};

use crate::states::{MultiSignatureVault, StreamProposal, ProposalStatus};

pub fn process_vote_on_proposal(accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {

    let [voter, stream_proposal_account, multisig_account, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !voter.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    };

    let proposal_id = u64::from_le_bytes(
        instruction_data[0..8].try_into().map_err(|_| ProgramError::InvalidInstructionData)?
    );

    let multisig_id = u64::from_le_bytes(
        instruction_data[8..16].try_into().map_err(|_| ProgramError::InvalidInstructionData)?
    );

    let vote_type = u64::from_le_bytes(
        instruction_data[16..24].try_into().map_err(|_| ProgramError::InvalidInstructionData)?
    );

    let (stream_proposal_account_pda, _bump) = pubkey::find_program_address(
        &[b"stream_proposal", proposal_id.to_le_bytes().as_ref(), multisig_id.to_le_bytes().as_ref()],
        &crate::ID
    );

    let multisig_account_info = MultiSignatureVault::from_account_info(multisig_account)?;

    let (multisig_info_pda, _multisig_info_bump) = pubkey::find_program_address(
        &[b"multisig_info", multisig_account_info.admin.as_ref(), multisig_id.to_le_bytes().as_ref()],
        &crate::ID
    );

    if *multisig_account.key() != multisig_info_pda {
        return Err(ProgramError::InvalidAccountData);
    };

    if *stream_proposal_account.key() != stream_proposal_account_pda {
        return Err(ProgramError::InvalidAccountData);
    };

    let mut stream_proposal_account_info = StreamProposal::from_account_info_mut(stream_proposal_account)?;

    let current_time = Clock::get()?.unix_timestamp;

    if current_time > stream_proposal_account_info.voting_deadline {
        return Err(ProgramError::Custom(2001)); 
    };

    if !multisig_account_info.is_active {
        return Err(ProgramError::Custom(2002)); 
    }

    let mut is_multisig_member = false;
    for member in multisig_account_info.member_keys {
        if *voter.key() == member {
            is_multisig_member = true;
            break; 
        }
    };

    if !is_multisig_member {
        return Err(ProgramError::InvalidAccountData);
    };

    let proposal_status = ProposalStatus::try_from(&stream_proposal_account_info.status)?;

    if proposal_status != ProposalStatus::Active {
        return Err(ProgramError::Custom(2003)); // Proposal not active
    };

    // Check for double voting before processing
    let voter_key = *voter.key();
    
    // Check if voter has already voted in either approval or rejection list
    for approval in stream_proposal_account_info.approvals {
        if approval == voter_key {
            return Err(ProgramError::Custom(2004)); 
        }
    }
    
    for rejection in stream_proposal_account_info.rejections {
        if rejection == voter_key {
            return Err(ProgramError::Custom(2004)); 
        }
    }

    match vote_type {
        0 => { 
            let mut member_added_to_approval_list = false;
            for i in 0..10 {
                if stream_proposal_account_info.approvals[i] == Pubkey::default() {
                    stream_proposal_account_info.approvals[i] = voter_key;
                    member_added_to_approval_list = true;
                    stream_proposal_account_info.approval_count = 
                        stream_proposal_account_info.approval_count.checked_add(1).unwrap();
                    stream_proposal_account_info.total_vote_count = 
                        stream_proposal_account_info.total_vote_count.checked_add(1).unwrap();
                    break; 
                }
            }

            if !member_added_to_approval_list {
                return Err(ProgramError::Custom(2005)); 
            }
        },
        1 => { 
            let mut member_added_to_rejection_list = false;
            for i in 0..10 {
                if stream_proposal_account_info.rejections[i] == Pubkey::default() {
                    stream_proposal_account_info.rejections[i] = voter_key;
                    member_added_to_rejection_list = true;
                    stream_proposal_account_info.rejection_count = 
                        stream_proposal_account_info.rejection_count.checked_add(1).unwrap();
                    stream_proposal_account_info.total_vote_count = 
                        stream_proposal_account_info.total_vote_count.checked_add(1).unwrap();
                    break; 
                }
            }

            if !member_added_to_rejection_list {
                return Err(ProgramError::Custom(2006)); 
            }
        },
        _ => {
            return Err(ProgramError::InvalidInstructionData); 
        },
    };

    if stream_proposal_account_info.approval_count >= stream_proposal_account_info.required_threshold {
        stream_proposal_account_info.status = ProposalStatus::Approved as u8;
    } else if stream_proposal_account_info.rejection_count >= stream_proposal_account_info.required_threshold {
        stream_proposal_account_info.status = ProposalStatus::Rejected as u8;
    }

    Ok(())
}