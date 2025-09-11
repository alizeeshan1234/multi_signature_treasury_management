use pinocchio::{account_info::AccountInfo, instruction::Signer, program_error::ProgramError, sysvars::{rent::Rent, Sysvar}, *};
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::{instructions::InitializeAccount3, state::TokenAccount};

use crate::states::PaymentStreamingInfo;

pub fn process_init_stream_payment(accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {

    let [sender, recipient, mint, sender_token_account, recipient_token_account, payment_stream_info, system_program, token_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !sender.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    };

    if *mint.owner() != pinocchio_token::ID {
        return Err(ProgramError::InvalidAccountData);
    };

    if instruction_data.len() != 24 {
        return Err(ProgramError::InvalidInstructionData);
    };

    let total_amount = u64::from_le_bytes(
        instruction_data[0..8].try_into().map_err(|_| ProgramError::InvalidInstructionData)?
    );

    let start_time = i64::from_le_bytes(
        instruction_data[8..16].try_into().map_err(|_| ProgramError::InvalidInstructionData)?
    );

    let end_time = i64::from_le_bytes(
        instruction_data[16..24].try_into().map_err(|_| ProgramError::InvalidInstructionData)?
    );

    if total_amount == 0 {
        msg!("Total amount cannot be zero");
        return Err(ProgramError::InvalidInstructionData);
    };

    if start_time >= end_time {
        msg!("Start time must be before end time");
        return Err(ProgramError::InvalidInstructionData);
    };

    let duration_seconds = end_time - start_time;
    let duration_hours = duration_seconds / 3600;

    if duration_hours <= 0 {
        msg!("Duration must be at least 1 hour");
        return Err(ProgramError::InvalidInstructionData);
    };

    let amount_per_hour = total_amount / (duration_hours as u64);
    
    let remainder = total_amount % (duration_hours as u64);
    if remainder != 0 {
        msg!("Warning: tokens will remain due to rounding");
    };

    let (payment_stream_info_pda, payment_stream_info_bump) = pubkey::find_program_address(
        &[b"payment_stream_info", sender.key().as_ref(), recipient.key().as_ref()],
        &crate::ID
    );

    if *payment_stream_info.key() != payment_stream_info_pda {
        return Err(ProgramError::InvalidAccountData);
    };

    let sender_token_account_info = TokenAccount::from_account_info(sender_token_account)?;
    if !sender_token_account_info.is_initialized() {
        return Err(ProgramError::UninitializedAccount);
    };

    if *sender_token_account_info.owner() != *sender.key() {
        return Err(ProgramError::InvalidAccountData);
    };

    if *sender_token_account_info.mint() != *mint.key() {
        return Err(ProgramError::InvalidAccountData);
    };

    if sender_token_account_info.amount() < total_amount {
        msg!("Insufficient token balance");
        return Err(ProgramError::InsufficientFunds);
    };

    let recipient_token_account_info = TokenAccount::from_account_info(recipient_token_account)?;

    if !recipient_token_account_info.is_initialized() {
        msg!("Initializing Recipient Token Account!");

        let lamports = Rent::get()?.minimum_balance(TokenAccount::LEN);

        CreateAccount {
            from: sender,
            to: recipient_token_account,
            lamports,
            space: TokenAccount::LEN as u64,
            owner: &pinocchio_token::ID,
        }.invoke()?;

        InitializeAccount3 {
            account: recipient_token_account,
            mint,
            owner: recipient.key()
        }.invoke()?;
    } else {
        if *recipient_token_account_info.owner() != *recipient.key() {
            return Err(ProgramError::InvalidAccountData);
        }

        if *recipient_token_account_info.mint() != *mint.key() {
            return Err(ProgramError::InvalidAccountData);
        }
    };

    if payment_stream_info.data_is_empty() {
        msg!("Initializing Payment Stream Info Account!");

        let lamports = Rent::get()?.minimum_balance(PaymentStreamingInfo::SIZE);

        let bump = &[payment_stream_info_bump];
        let seeds = seeds!(
            b"payment_stream_info", 
            sender.key().as_ref(), 
            recipient.key().as_ref(),
            bump
        );
        let signer_seeds = Signer::from(&seeds);

        CreateAccount {
            from: sender,
            to: payment_stream_info,
            lamports,
            space: PaymentStreamingInfo::SIZE as u64,
            owner: &crate::ID
        }.invoke_signed(&[signer_seeds])?;

        let mut payment_stream_info_account = PaymentStreamingInfo::from_account_info_mut(payment_stream_info)?;
        payment_stream_info_account.sender = *sender.key();
        payment_stream_info_account.recipient = *recipient.key();
        payment_stream_info_account.sender_token_account = *sender_token_account.key();
        payment_stream_info_account.recipient_token_account = *recipient_token_account.key();
        payment_stream_info_account.total_amount = total_amount; // ✅ Added this
        payment_stream_info_account.amount_per_hour = amount_per_hour;
        payment_stream_info_account.start_time = start_time;
        payment_stream_info_account.end_time = end_time;
        payment_stream_info_account.withdrawn_amount = 0;
        payment_stream_info_account.is_active = true;

        msg!("Payment Stream Account Initialized Successfully!");
    } else {
        msg!("Payment Stream Account Info Already Exists");
        return Err(ProgramError::AccountAlreadyInitialized);
    };

    Ok(())
}