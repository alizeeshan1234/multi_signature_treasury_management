use pinocchio::{account_info::AccountInfo, instruction::Signer, program_error::ProgramError, sysvars::{rent::Rent, Sysvar}, *};
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::{instructions::TransferChecked, state::{Mint, TokenAccount}};
use pinocchio_token::instructions::InitializeAccount3;

pub fn process_transfer_tokens(accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {

    let [sender, recepient, mint, sender_token_account, recepient_token_account, token_program, system_program] = accounts else {
        return Err(ProgramError::InvalidAccountData);
    };

    if !sender.is_signer() {
        return Err(ProgramError::InvalidAccountData);
    };

    if *mint.owner() != pinocchio_token::ID {
        return Err(ProgramError::InvalidAccountData);
    };

    let amount_to_transfer = u64::from_le_bytes(
        instruction_data[0..8].try_into().map_err(|_| ProgramError::InvalidInstructionData)?
    );

    let sender_token_account_info = TokenAccount::from_account_info(sender_token_account)?;

    if !sender_token_account_info.is_initialized() {
        return Err(ProgramError::InvalidAccountData);
    };

    if sender_token_account_info.owner() != sender.key() {
        return Err(ProgramError::InvalidAccountData);
    };

    if sender_token_account_info.mint() != mint.key() {
        return Err(ProgramError::InvalidAccountData);
    };

    if sender_token_account_info.amount() < amount_to_transfer {
        return Err(ProgramError::InsufficientFunds);
    };

    let (recepient_token_account_pda, bump) = pubkey::find_program_address(
        &[b"recepient_token_account", recepient.key().as_ref(), mint.key().as_ref()],
        &crate::ID
    );

    if *recepient_token_account.key() != recepient_token_account_pda {
        return Err(ProgramError::InvalidAccountData);
    };

    let recepient_token_account_info = TokenAccount::from_account_info(recepient_token_account)?;

    if !recepient_token_account_info.is_initialized() {
        msg!("Initializing Recipient Token Account");

        let lamports = Rent::get()?.minimum_balance(TokenAccount::LEN);

        let bump_ref = &[bump];
        let seeds = seeds!(
            b"recepient_token_account",
            recepient.key().as_ref(), 
            mint.key().as_ref(),
            bump_ref
        );
        let signer_seeds = Signer::from(&seeds);

        CreateAccount {
            from: sender,
            to: recepient_token_account,
            lamports,
            space: TokenAccount::LEN as u64,
            owner: &pinocchio_token::ID 
        }.invoke_signed(&[signer_seeds])?;

        InitializeAccount3 {
            account: recepient_token_account,
            mint,
            owner: recepient.key(),
        }.invoke()?;
    };

    let mint_info = Mint::from_account_info(mint)?;

    TransferChecked {
        from: sender_token_account,
        mint,
        to: recepient_token_account,
        authority: sender,
        amount: amount_to_transfer,
        decimals: mint_info.decimals()
    }.invoke()?;

    Ok(())
}