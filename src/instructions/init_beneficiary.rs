use pinocchio::{account_info::AccountInfo, instruction::Signer, program_error::ProgramError, sysvars::{rent::Rent, Sysvar}, *};
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::{instructions::InitializeAccount3, state::TokenAccount};

use crate::states::{VestingContractInfo, VestingRecordInfo};

pub fn process_init_beneficiary(accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {

    let [creator, beneficiary, mint, vesting_contract_info, vesting_record_info, beneficiary_token_account, system_program, token_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !creator.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    };

    if instruction_data.len() < 64 {
        return Err(ProgramError::InvalidInstructionData);
    }

    let mut company_name = [0u8; 32];
    company_name.copy_from_slice(&instruction_data[0..32]);

    let cliff_period = i64::from_le_bytes(
        instruction_data[32..40].try_into().map_err(|_| ProgramError::InvalidInstructionData)?
    );

    let start_time = i64::from_le_bytes(
        instruction_data[40..48].try_into().map_err(|_| ProgramError::InvalidInstructionData)?
    );

    let end_time = i64::from_le_bytes(
        instruction_data[48..56].try_into().map_err(|_| ProgramError::InvalidInstructionData)?
    );

    let vesting_amount = u64::from_le_bytes(
        instruction_data[56..64].try_into().map_err(|_| ProgramError::InvalidInstructionData)?
    );

    if start_time >= end_time {
        msg!("Start time must be before end time");
        return Err(ProgramError::InvalidInstructionData);
    }

    if vesting_amount == 0 {
        msg!("Vesting amount cannot be zero");
        return Err(ProgramError::InvalidInstructionData);
    }

    if cliff_period < 0 {
        msg!("Cliff period cannot be negative");
        return Err(ProgramError::InvalidInstructionData);
    }

    let (vesting_contract_info_pda, vesting_contract_bump) = pubkey::find_program_address(
        &[company_name.as_ref(), creator.key().as_ref()],
        &crate::ID
    );

    if *vesting_contract_info.key() != vesting_contract_info_pda {
        return Err(ProgramError::InvalidAccountData);
    };

    let (vesting_record_info_pda, vesting_record_info_bump) = pubkey::find_program_address(
        &[b"vesting_record_info", vesting_contract_info.key().as_ref(), beneficiary.key().as_ref()],
        &crate::ID
    );

    if *vesting_record_info.key() != vesting_record_info_pda {
        return Err(ProgramError::InvalidAccountData);
    };

    let (beneficiary_token_account_pda, beneficiary_token_account_pda_bump) = pubkey::find_program_address(
        &[beneficiary.key().as_ref(), mint.key().as_ref()],
        &crate::ID
    );

    if *beneficiary_token_account.key() != beneficiary_token_account_pda {
        return Err(ProgramError::InvalidAccountData);
    };

    let mut vesting_contract_info_account_mut = VestingContractInfo::from_account_info_mut(vesting_contract_info)?;

    let beneficiary_token_account_info = TokenAccount::from_account_info(beneficiary_token_account)?;

    if *mint.owner() != pinocchio_token::ID {
        return Err(ProgramError::InvalidAccountData);
    }

    if vesting_contract_info_account_mut.creator != *creator.key() {
        return Err(ProgramError::InvalidAccountData);
    }

    if !vesting_contract_info_account_mut.is_active {
        msg!("Vesting contract is not active");
        return Err(ProgramError::InvalidAccountData);
    }

    if !beneficiary_token_account_info.is_initialized() {
        msg!("Initializing Beneficiary Token Account");

        let lamports = Rent::get()?.minimum_balance(TokenAccount::LEN);

        let bump = &[beneficiary_token_account_pda_bump];
        let seeds = seeds!(
            beneficiary.key().as_ref(), 
            mint.key().as_ref(),
            bump
        );
        let signer_seeds = Signer::from(&seeds);

        CreateAccount {
            from: creator,
            to: beneficiary_token_account,
            lamports,
            space: TokenAccount::LEN as u64,
            owner: &pinocchio_token::ID
        }.invoke_signed(&[signer_seeds])?;

        InitializeAccount3 {
            account: beneficiary_token_account,
            mint,
            owner: beneficiary.key(), 
        }.invoke()?;
    }

    if vesting_record_info.data_is_empty() {
        msg!("Creating Vesting Record Info Account!");

        let lamports = Rent::get()?.minimum_balance(VestingRecordInfo::SIZE);

        let bump = &[vesting_record_info_bump];
        let seeds = seeds!(
            b"vesting_record_info", 
            vesting_contract_info.key().as_ref(), 
            beneficiary.key().as_ref(),
            bump
        );
        let signer_seeds = Signer::from(&seeds);

        CreateAccount {
            from: creator,
            to: vesting_record_info,
            lamports,
            space: VestingRecordInfo::SIZE as u64,
            owner: &crate::ID
        }.invoke_signed(&[signer_seeds])?;

        let mut vesting_record_info_mut = VestingRecordInfo::from_account_info_mut(vesting_record_info)?;
        vesting_record_info_mut.beneficiary = *beneficiary.key();
        vesting_record_info_mut.mint = *mint.key();
        vesting_record_info_mut.total_vested_tokens = vesting_amount;
        vesting_record_info_mut.total_claimed_tokens_by_beneficiary = 0;
        vesting_record_info_mut.cliff_period = cliff_period;
        vesting_record_info_mut.vault_bump = vesting_contract_info_account_mut.vault_bump;
        vesting_record_info_mut.vesting_contract_info_bump = vesting_contract_info_account_mut.bump;
        vesting_record_info_mut.bump = vesting_record_info_bump;
        vesting_record_info_mut.beneficiary_ata = *beneficiary_token_account.key();
        vesting_record_info_mut.has_claimed = false;
        vesting_record_info_mut.start_time = start_time;
        vesting_record_info_mut.end_time = end_time;
    }

    vesting_contract_info_account_mut.total_available_tokens += vesting_amount;
    vesting_contract_info_account_mut.total_locked_tokens += vesting_amount;

    Ok(())
}