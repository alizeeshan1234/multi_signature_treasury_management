use pinocchio::{account_info::AccountInfo, instruction::Signer, program_error::ProgramError, sysvars::{clock::Clock, Sysvar}, *};
use pinocchio_token::instructions::Transfer;

use crate::states::PaymentStreamingInfo;

pub fn process_withdraw_stream_payments(accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {

    let [recipient, sender_token_account, recipient_token_account, payment_stream_info, token_program] = accounts else {
        return Err(ProgramError::InvalidAccountData);
    };

    if !recipient.is_signer() {
        return Err(ProgramError::InvalidAccountData);
    };

    let mut payment_stream_account_info = PaymentStreamingInfo::from_account_info_mut(payment_stream_info)?;

    if !payment_stream_account_info.is_active {
        msg!("Payment stream is not active");
        return Err(ProgramError::InvalidAccountData);
    };

    if *recipient.key() != payment_stream_account_info.recipient {
        msg!("Only the recipient can withdraw from the stream");
        return Err(ProgramError::InvalidAccountData);
    };

    if *sender_token_account.key() != payment_stream_account_info.sender_token_account {
        return Err(ProgramError::InvalidAccountData);
    };

    if *recipient_token_account.key() != payment_stream_account_info.recipient_token_account {
        return Err(ProgramError::InvalidAccountData);
    };

    let current_time = Clock::get()?.unix_timestamp;

    if current_time < payment_stream_account_info.start_time {
        msg!("Stream not started yet!");
        return Err(ProgramError::InvalidAccountData);
    };

    let withdrawable_amount = calculate_withdrawable_amount(&payment_stream_account_info, current_time)?;

    if withdrawable_amount == 0 {
        msg!("No tokens available for withdrawal");
        return Err(ProgramError::InvalidInstructionData);
    };

    let (payment_stream_info_pda, payment_stream_info_bump) = pubkey::find_program_address(
        &[
            b"payment_stream_info", 
            payment_stream_account_info.sender.as_ref(), 
            payment_stream_account_info.recipient.as_ref()
        ],
        &crate::ID
    );

    if *payment_stream_info.key() != payment_stream_info_pda {
        return Err(ProgramError::InvalidAccountData);
    }

    let bump = &[payment_stream_info_bump];
    let seeds = seeds!(
        b"payment_stream_info", 
        payment_stream_account_info.sender.as_ref(), 
        payment_stream_account_info.recipient.as_ref(),
        bump
    );
    let signer_seeds = Signer::from(&seeds);

    Transfer {
        from: sender_token_account,
        to: recipient_token_account,
        authority: payment_stream_info, 
        amount: withdrawable_amount,
    }.invoke_signed(&[signer_seeds])?;

    payment_stream_account_info.withdrawn_amount += withdrawable_amount;

    if current_time >= payment_stream_account_info.end_time {
        payment_stream_account_info.is_active = false;
        msg!("Stream completed and deactivated");
    }

    Ok(())
}

fn calculate_withdrawable_amount(stream_info: &PaymentStreamingInfo, current_time: i64) -> Result<u64, ProgramError> {

    let start_time = stream_info.start_time;
    let end_time = stream_info.end_time;
    let total_amount = stream_info.total_amount;
    let withdrawn_amount = stream_info.withdrawn_amount;

    let effective_current_time = if current_time > end_time {
        end_time
    } else {
        current_time
    };

    let elapsed_seconds = effective_current_time - start_time;
    let total_duration_seconds = end_time - start_time;

    if elapsed_seconds <= 0 {
        return Ok(0);
    };

    let total_available = if effective_current_time >= end_time {
        total_amount
    } else {
        (total_amount as u128 * elapsed_seconds as u128 / total_duration_seconds as u128) as u64
    };

    if total_available > withdrawn_amount {
        Ok(total_available - withdrawn_amount)
    } else {
        Ok(0)
    }
}