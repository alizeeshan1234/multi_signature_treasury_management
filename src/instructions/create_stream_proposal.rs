use pinocchio::{account_info::AccountInfo, instruction::Signer, program_error::ProgramError, pubkey::Pubkey, sysvars::{clock::Clock, rent::Rent, Sysvar}, *};
use pinocchio_system::instructions::CreateAccount;

use crate::states::{MultiSignatureVault, StreamProposal, StreamType, ProposalStatus};

pub fn process_create_stream_proposal(accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {

    if accounts.len() < 4 {
        return Err(ProgramError::InvalidAccountData);
    };

    let [proposer, stream_proposal_account, multisig_account, system_program] = accounts else {
        return Err(ProgramError::InvalidAccountData);
    };

    if !proposer.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if instruction_data.len() < 186 {
        return Err(ProgramError::InvalidInstructionData);
    };

    let proposal_id = u64::from_le_bytes(
        instruction_data[0..8].try_into().map_err(|_| ProgramError::InvalidInstructionData)?
    );

    let multisig_id = u64::from_le_bytes(
        instruction_data[8..16].try_into().map_err(|_| ProgramError::InvalidInstructionData)?
    );

    let stream_type_raw = instruction_data[16];

    let required_threshold = instruction_data[17];

    let voting_deadline = i64::from_le_bytes(
        instruction_data[18..26].try_into().map_err(|_| ProgramError::InvalidInstructionData)?
    );

    let mut stream_name = [0u8; 32];
    stream_name.copy_from_slice(&instruction_data[26..58]);

    let mut stream_description = [0u8; 128];
    stream_description.copy_from_slice(&instruction_data[58..186]);

    let _stream_type = StreamType::try_from(&stream_type_raw)?;

    let current_time = Clock::get()?.unix_timestamp;
    if voting_deadline <= current_time {
        return Err(ProgramError::InvalidInstructionData);
    }

    // Load and validate multisig account
    let multisig_account_info = MultiSignatureVault::from_account_info(multisig_account)?;
    
    let (multisig_info_pda, _) = pubkey::find_program_address(
        &[b"multisig_info", multisig_account_info.admin.as_ref(), multisig_id.to_le_bytes().as_ref()],
        &crate::ID
    );

    if *multisig_account.key() != multisig_info_pda {
        return Err(ProgramError::InvalidAccountData);
    }

    // Validate proposer is a member of the multisig
    let mut is_multisig_member = false;
    for member in multisig_account_info.member_keys {
        if *proposer.key() == member {
            is_multisig_member = true;
            break;
        }
    }

    if !is_multisig_member {
        return Err(ProgramError::InvalidInstructionData);
    }

    if required_threshold == 0 || required_threshold as u64 > multisig_account_info.member_count {
        return Err(ProgramError::InvalidInstructionData);
    }

    let (stream_proposal_account_pda, bump) = pubkey::find_program_address(
        &[b"stream_proposal", proposal_id.to_le_bytes().as_ref(), multisig_id.to_le_bytes().as_ref()],
        &crate::ID
    );

    if *stream_proposal_account.key() != stream_proposal_account_pda {
        return Err(ProgramError::InvalidAccountData);
    };

    if stream_proposal_account.data_is_empty() {
        let lamports = Rent::get()?.minimum_balance(StreamProposal::SIZE);

        let proposal_id_ref = proposal_id.to_le_bytes();
        let multisig_id_ref = multisig_id.to_le_bytes();
        let bump_ref = &[bump];
        let seeds = seeds!(
            b"stream_proposal",
            proposal_id_ref.as_ref(),
            multisig_id_ref.as_ref(),
            bump_ref
        );
        let signer_seeds = Signer::from(&seeds);

        CreateAccount {
            from: proposer,
            to: stream_proposal_account,
            lamports,
            space: StreamProposal::SIZE as u64,
            owner: &crate::ID
        }.invoke_signed(&[signer_seeds])?;

        let mut stream_proposal_account_info = StreamProposal::from_account_info_mut(stream_proposal_account)?;
        
        stream_proposal_account_info.proposer = *proposer.key();
        stream_proposal_account_info.stream_name = stream_name;
        stream_proposal_account_info.stream_description = stream_description;
        stream_proposal_account_info.proposal_id = proposal_id;
        stream_proposal_account_info.multisig_id = multisig_id;
        stream_proposal_account_info.stream_type = stream_type_raw;
        stream_proposal_account_info.created_at = current_time;
        stream_proposal_account_info.voting_deadline = voting_deadline;
        stream_proposal_account_info.approvals = [Pubkey::default(); 10];
        stream_proposal_account_info.approval_count = 0;
        stream_proposal_account_info.rejections = [Pubkey::default(); 10];
        stream_proposal_account_info.rejection_count = 0;
        stream_proposal_account_info.total_vote_count = 0;
        stream_proposal_account_info.required_threshold = required_threshold;
        stream_proposal_account_info.status = ProposalStatus::Active as u8;
        
    } else {
        return Err(ProgramError::AccountAlreadyInitialized);
    }
    
    Ok(())
}