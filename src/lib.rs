use pinocchio::{account_info::AccountInfo, entrypoint, ProgramResult, pubkey::Pubkey, program_error::ProgramError};
use pinocchio_pubkey::declare_id;

use crate::instructions::MultiSignatureInstructions;

declare_id!("4taWcHcTu9CbPv6JiB9HNxU9aQrni7dARoDtyv9GsWQ6");

entrypoint!(process_instruction);

pub mod instructions;
pub mod states;

fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8]
) -> ProgramResult {

    let (ix_disc, instruction_data) = instruction_data.split_first().ok_or(ProgramError::InvalidInstructionData)?;

    match MultiSignatureInstructions::try_from(ix_disc)? {
        MultiSignatureInstructions::InitMultisigVault => instructions::init_multisig_vault::process_init_multisig_vault(accounts, instruction_data)?,
        MultiSignatureInstructions::AddMember => instructions::add_members::process_add_member(accounts, instruction_data)?,
        MultiSignatureInstructions::CreateStreamProposal => instructions::create_stream_proposal::process_create_stream_proposal(accounts, instruction_data)?,
    }

    Ok(())
}