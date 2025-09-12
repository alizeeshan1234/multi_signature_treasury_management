use pinocchio::{account_info::AccountInfo, instruction::Signer, program_error::ProgramError, sysvars::{clock::Clock, Sysvar}, *};
use pinocchio_token::{instructions::TransferChecked, state::Mint};

use crate::states::{VestingContractInfo, VestingRecordInfo};

pub fn process_claim_vested_tokens(accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {

    let [beneficiary, creator, mint, vesting_contract_info, vesting_record_info, vault, beneficiary_token_account, system_program, token_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !beneficiary.is_signer() {
        return Err(ProgramError::InvalidAccountData);
    };

    let mut company_name = [0u8; 32];
    company_name.copy_from_slice(&instruction_data[0..32]);

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

    let (vault_pda, vault_bump) = pubkey::find_program_address(
        &[b"vault", mint.key().as_ref(), creator.key().as_ref()],
        &crate::ID
    );

    if *vault.key() != vault_pda {
        return Err(ProgramError::InvalidAccountData);
    };

    let (beneficiary_token_account_pda, beneficiary_token_account_pda_bump) = pubkey::find_program_address(
        &[beneficiary.key().as_ref(), mint.key().as_ref()],
        &crate::ID
    );

    if *beneficiary_token_account.key() != beneficiary_token_account_pda {
        return Err(ProgramError::InvalidAccountData);
    };

    let mut vesting_contract_info_mut = VestingContractInfo::from_account_info_mut(vesting_contract_info)?;
    let mut vesting_record_info_mut = VestingRecordInfo::from_account_info_mut(vesting_record_info)?;
    let current_time = Clock::get()?.unix_timestamp;

    if current_time < vesting_record_info_mut.start_time {
        return Err(ProgramError::InvalidAccountData);
    };

    if vesting_contract_info_mut.is_active == false {
        return Err(ProgramError::InvalidAccountData);
    };

    let total_vested_tokens = vesting_record_info_mut.total_vested_tokens;
    let total_claimed_tokens = vesting_record_info_mut.total_claimed_tokens_by_beneficiary;

    let unlock_time = vesting_record_info_mut.start_time.checked_add(vesting_record_info_mut.cliff_period).ok_or(ProgramError::InvalidAccountData)?;

    if current_time < unlock_time {
        return Err(ProgramError::InvalidAccountData);
    };

    let claimable_tokens = if current_time >= vesting_record_info_mut.end_time {
        total_vested_tokens.checked_sub(total_claimed_tokens).ok_or(ProgramError::InvalidAccountData)?
    } else {
        let total_vesting_duration = vesting_record_info_mut.end_time.checked_sub(vesting_record_info_mut.start_time).ok_or(ProgramError::InvalidAccountData)?;
        let elapsed_time = current_time.checked_sub(vesting_record_info_mut.start_time).ok_or(ProgramError::InvalidAccountData)?;
        
        let total_unlocked = (total_vested_tokens as u128)
            .checked_mul(elapsed_time as u128)
            .ok_or(ProgramError::InvalidAccountData)?
            .checked_div(total_vesting_duration as u128)
            .ok_or(ProgramError::InvalidAccountData)? as u64;
            
        total_unlocked.checked_sub(total_claimed_tokens).ok_or(ProgramError::InvalidAccountData)?
    };

    if claimable_tokens == 0 {
        return Err(ProgramError::InvalidAccountData); 
    };

    let company_name = vesting_contract_info_mut.company_name;
    let creator_key = *creator.key();

    let bump = &[vesting_contract_bump];
    let seeds = seeds!(
        company_name.as_ref(), 
        creator_key.as_ref(),
        bump
    );
    let signer_seeds = Signer::from(&seeds);

    let mint_account_info = Mint::from_account_info(mint)?;

    TransferChecked {
        from: vault,
        to: beneficiary_token_account,
        mint: mint,
        authority: vesting_contract_info,
        amount: claimable_tokens,
        decimals: mint_account_info.decimals(),
    }.invoke_signed(&[signer_seeds])?;

    vesting_record_info_mut.total_claimed_tokens_by_beneficiary = total_vested_tokens; 
    vesting_record_info_mut.has_claimed = true; 
    vesting_contract_info_mut.total_claimed_tokens = vesting_contract_info_mut.total_claimed_tokens.checked_add(claimable_tokens).ok_or(ProgramError::InvalidAccountData)?;

    if vesting_contract_info_mut.total_claimed_tokens >= vesting_contract_info_mut.total_vested_tokens {
        vesting_contract_info_mut.fully_claimed = true;
    }

    Ok(())
}