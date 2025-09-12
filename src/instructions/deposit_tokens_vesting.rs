use pinocchio::{account_info::AccountInfo, instruction::Signer, program_error::ProgramError, *};
use pinocchio_token::{instructions::TransferChecked, state::{Mint, TokenAccount}};

use crate::states::VestingContractInfo;

pub fn process_deposit_token_vesting_vault(accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {

    let [creator, mint, vesting_contract_info, vault, company_token_account, system_program, token_program] = accounts else {
        return Err(ProgramError::InvalidAccountData);
    };

    if !creator.is_signer() {
        return Err(ProgramError::InvalidAccountData);
    };

    if *mint.owner() != pinocchio_token::ID {
        return Err(ProgramError::InvalidAccountData);
    };

    let mut company_name = [0u8; 32];
    company_name.copy_from_slice(&instruction_data[0..32]);

    let deposit_amount = u64::from_le_bytes(
        instruction_data[32..40].try_into().map_err(|_| ProgramError::InvalidAccountData)?
    );

    let mut vesting_contract_account_info = VestingContractInfo::from_account_info_mut(vesting_contract_info)?;

    let (vesting_contract_info_pda, vesting_contract_bump) = pubkey::find_program_address(
        &[company_name.as_ref(), creator.key().as_ref()],
        &crate::ID
    );

    if *vesting_contract_info.key() != vesting_contract_info_pda {
        return Err(ProgramError::InvalidAccountData);
    };

    let (vault_pda, vault_bump) = pubkey::find_program_address(
        &[b"vault", mint.key().as_ref(), creator.key().as_ref()],
        &crate::ID
    );

    if *vault.key() != vault_pda {
        return Err(ProgramError::InvalidAccountData);
    };

    let company_token_account_info = TokenAccount::from_account_info(company_token_account)?;

    if company_token_account_info.amount() < deposit_amount {
        return Err(ProgramError::InsufficientFunds);
    };

    if *company_token_account_info.owner() != *creator.key() {
        return Err(ProgramError::IllegalOwner);
    };

    let mint_account_info = Mint::from_account_info(mint)?;

    TransferChecked {
        from: company_token_account,
        to: vault,
        mint: mint,
        authority: creator, 
        amount: deposit_amount,
        decimals: mint_account_info.decimals()
    }.invoke()?;

    vesting_contract_account_info.total_vested_tokens += deposit_amount;
    vesting_contract_account_info.total_available_tokens += deposit_amount;

    Ok(())
}