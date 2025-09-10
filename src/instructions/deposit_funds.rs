use pinocchio::{
    account_info::AccountInfo, 
    instruction::Signer, 
    program_error::ProgramError, 
    pubkey::Pubkey, 
    sysvars::{clock::Clock, rent::Rent, Sysvar}, 
    *
};
use pinocchio_token::state::{Mint, TokenAccount};
use pinocchio_token::instructions::TransferChecked;

use crate::states::MultiSignatureVault;

pub fn deposit_funds_to_treasury(accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {

    let [admin, mint, source_token_account, treasury_vault, multisig_account, token_program, system_program] = accounts else {
        return Err(ProgramError::InvalidAccountData);
    };

    if !admin.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if *token_program.key() != pinocchio_token::ID {
        return Err(ProgramError::IncorrectProgramId);
    }
    
    let multisig_id = u64::from_le_bytes(
        instruction_data[0..8]
            .try_into()
            .map_err(|_| ProgramError::InvalidInstructionData)?
    );

    let amount = u64::from_le_bytes(
        instruction_data[8..16]
            .try_into()
            .map_err(|_| ProgramError::InvalidInstructionData)?
    );

    if amount == 0 {
        return Err(ProgramError::InvalidInstructionData);
    }

    if multisig_account.data_is_empty() {
        return Err(ProgramError::UninitializedAccount);
    }

    let multisig_account_info = MultiSignatureVault::from_account_info(multisig_account)?;
    if multisig_account_info.admin != *admin.key() {
        return Err(ProgramError::InvalidAccountData);
    }

    let (multisig_info_pda, _) = pubkey::find_program_address(
        &[
            b"multisig_info", 
            admin.key().as_ref(), 
            multisig_id.to_le_bytes().as_ref()
        ],
        &crate::ID
    );

    if *multisig_account.key() != multisig_info_pda {
        return Err(ProgramError::InvalidAccountData);
    }

    let (treasury_vault_pda, _) = pubkey::find_program_address(
        &[
            b"multisig_vault", 
            mint.key().as_ref(), 
            multisig_id.to_le_bytes().as_ref()
        ],
        &crate::ID
    );

    if *treasury_vault.key() != treasury_vault_pda {
        return Err(ProgramError::InvalidAccountData);
    }

    let source_token_account_info = TokenAccount::from_account_info(source_token_account)?;
    if !source_token_account_info.is_initialized() {
        return Err(ProgramError::InvalidAccountData);
    }

    let treasury_vault_account_info = TokenAccount::from_account_info(treasury_vault)?;
    if !treasury_vault_account_info.is_initialized() {
        return Err(ProgramError::InvalidAccountData);
    }

    if *source_token_account_info.owner() != *admin.key() {
        return Err(ProgramError::InvalidAccountData);
    }

    if *treasury_vault_account_info.owner() != multisig_info_pda {
        return Err(ProgramError::InvalidAccountData);
    }

    if *source_token_account_info.mint() != *mint.key() {
        return Err(ProgramError::InvalidAccountData);
    }

    if *treasury_vault_account_info.mint() != *mint.key() {
        return Err(ProgramError::InvalidAccountData);
    }

    if source_token_account_info.amount() < amount {
        return Err(ProgramError::InsufficientFunds);
    }

    let mint_account_info = Mint::from_account_info(mint)?;

    TransferChecked {
        from: source_token_account,
        mint,
        to: treasury_vault,
        authority: admin,
        amount,
        decimals: mint_account_info.decimals(),
    }.invoke()?;

    Ok(())
}

