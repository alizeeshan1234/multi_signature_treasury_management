use pinocchio::{account_info::AccountInfo, instruction::Signer, program_error::ProgramError, sysvars::{clock::Clock, Sysvar}, ProgramResult, *};
use pinocchio_token::{instructions::{SetAuthority, Transfer}, state::TokenAccount};
use pinocchio_token::instructions::AuthorityType;

use crate::{instructions::withdraw_stream_payments::calculate_withdrawable_amount, states::PaymentStreamingInfo};

pub fn process_cancel_stream(accounts: &[AccountInfo]) -> ProgramResult {

    let [sender, recipient, sender_token_account, recipient_token_account, payment_stream_info_account, token_program, system_program] = accounts else {
       return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !sender.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    };

    let mut stream_account_info_mut = PaymentStreamingInfo::from_account_info_mut(payment_stream_info_account)?;

    if !stream_account_info_mut.is_active {
        msg!("Payment stream is not active");
        return Err(ProgramError::InvalidAccountData);
    }

    if *sender.key() != stream_account_info_mut.sender {
        return Err(ProgramError::InvalidAccountData);
    }

    if *recipient.key() != stream_account_info_mut.recipient {
        return Err(ProgramError::InvalidAccountData);
    }

    let (payment_stream_info_pda, payment_stream_info_bump) = pubkey::find_program_address(
        &[b"payment_stream_info", sender.key().as_ref(), recipient.key().as_ref()],
        &crate::ID
    );

    if *payment_stream_info_account.key() != payment_stream_info_pda {
        return Err(ProgramError::InvalidAccountData);
    };

    let sender_token_account_info = TokenAccount::from_account_info(sender_token_account)?;
    if !sender_token_account_info.is_initialized() {
        return Err(ProgramError::UninitializedAccount);
    };
    
    let recipient_token_account_info = TokenAccount::from_account_info(recipient_token_account)?;
    if !recipient_token_account_info.is_initialized() {
        return Err(ProgramError::UninitializedAccount);
    };

    let current_time = Clock::get()?.unix_timestamp;

    let withdrawable_for_recipient = calculate_withdrawable_amount(&stream_account_info_mut, current_time)?;

    let bump_ref = &[payment_stream_info_bump];
    let seeds = seeds!(
        b"payment_stream_info", 
        sender.key().as_ref(), 
        recipient.key().as_ref(),
        bump_ref
    );
    let signer_seeds = Signer::from(&seeds);
    let signer_seeds_clone = signer_seeds.clone();

    if withdrawable_for_recipient > 0 {
        Transfer {
            from: sender_token_account,
            to: recipient_token_account,
            authority: payment_stream_info_account,
            amount: withdrawable_for_recipient,
        }.invoke_signed(&[signer_seeds])?;
    }

    stream_account_info_mut.is_active = false;
    stream_account_info_mut.withdrawn_amount += withdrawable_for_recipient;

    SetAuthority {
        account: sender_token_account,
        authority: payment_stream_info_account,
        authority_type: AuthorityType::AccountOwner,
        new_authority: Some(sender.key()), 
    }.invoke_signed(&[signer_seeds_clone])?;

    Ok(())
}