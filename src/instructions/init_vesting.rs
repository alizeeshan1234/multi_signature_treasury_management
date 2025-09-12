use pinocchio::{account_info::AccountInfo, instruction::Signer, program_error::ProgramError, sysvars::{clock::Clock, rent::Rent, Sysvar}, ProgramResult,*};
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::{instructions::InitializeAccount3, state::TokenAccount};

use crate::states::VestingContractInfo;

pub fn process_init_vesting_contract(accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {

    let [creator, mint, vesting_contract_info, vault, comapny_token_account, system_program, token_program] = accounts else {
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

    let (company_token_account_pda, company_token_account_bump) = pubkey::find_program_address(
        &[mint.key().as_ref(), creator.key().as_ref()],
        &crate::ID
    );

    if *comapny_token_account.key() != company_token_account_pda {
        return Err(ProgramError::InvalidAccountData);
    };

    let company_token_account_info = TokenAccount::from_account_info(comapny_token_account)?;
    if !company_token_account_info.is_initialized() {
        return Err(ProgramError::InvalidAccountData);
    };

    let vault_account_info = TokenAccount::from_account_info(vault)?;

    if !vault_account_info.is_initialized() {
        msg!("Initializing Vault Account");

        let lamports = Rent::get()?.minimum_balance(TokenAccount::LEN);

        let vault_bump_ref = &[vault_bump];
        let vault_seeds = seeds!(
            b"vault", 
            mint.key().as_ref(), 
            creator.key().as_ref(),
            vault_bump_ref
        );
        let vault_signer = Signer::from(&vault_seeds);

        CreateAccount {
            from: creator,
            to: vault,
            lamports,
            space: TokenAccount::LEN as u64,
            owner: &pinocchio_token::ID
        }.invoke_signed(&[vault_signer])?;

        InitializeAccount3 {
            account: vault,
            mint,
            owner: &vesting_contract_info_pda
        }.invoke()?;
    };

    if vesting_contract_info.data_is_empty() {
        msg!("Creating Vesting Contract Info Account");

        let lamports = Rent::get()?.minimum_balance(VestingContractInfo::SIZE);

        let vesting_bump_ref = &[vesting_contract_bump];
        let vesting_seeds = seeds!(
            company_name.as_ref(),
            creator.key().as_ref(),
            vesting_bump_ref
        );  
        let vesting_signer = Signer::from(&vesting_seeds);

        CreateAccount {
            from: creator,
            to: vesting_contract_info,
            lamports,
            space: VestingContractInfo::SIZE as u64,
            owner: &crate::ID
        }.invoke_signed(&[vesting_signer])?;

        let mut vesting_contract_info_mut = VestingContractInfo::from_account_info_mut(vesting_contract_info)?;

        vesting_contract_info_mut.creator = *creator.key();
        vesting_contract_info_mut.company_name = company_name;
        vesting_contract_info_mut.mint = *mint.key();
        vesting_contract_info_mut.total_vested_tokens = 0;
        vesting_contract_info_mut.total_available_tokens = 0;
        vesting_contract_info_mut.total_locked_tokens = 0;
        vesting_contract_info_mut.total_claimed_tokens = 0;
        vesting_contract_info_mut.vault_account = *vault.key();
        vesting_contract_info_mut.created_at = Clock::get()?.unix_timestamp;
        vesting_contract_info_mut.vault_bump = vault_bump;
        vesting_contract_info_mut.bump = vesting_contract_bump;
        vesting_contract_info_mut.is_active = true;
        vesting_contract_info_mut.fully_claimed = false;
    }

    Ok(())
}