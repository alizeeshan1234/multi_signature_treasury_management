use pinocchio::program_error::ProgramError;
use shank::ShankInstruction;

pub mod init_multisig_vault;
pub mod add_members;
pub mod create_stream_proposal;

#[repr(u8)]
#[derive(ShankInstruction)]
pub enum MultiSignatureInstructions {
    #[account(0, writable, signer, name="admin", desc="Account that pays for account creation")]
    #[account(1, name="mint", desc="mint account")]
    #[account(2, writable, name="multisig_info", desc="multisig_info account")]
    #[account(3, writable, name="treasury_vault_account", desc="treasury vault account")]
    #[account(4, name="token_program", desc="TokenProgram")]
    #[account(5, name="system_program", desc="System program")]
    InitMultisigVault = 0, //Create multi-sign vault (name, description, members, threshold)
    
    #[account(0, writable, signer, name="admin", desc="Account that pays for account creation")]
    #[account(1, name="member", desc="member to add")]
    #[account(2, writable, name="multisig_info", desc="multisig_info account")]
    #[account(3, name="system_program", desc="System program")]
    AddMember = 1, //Add new member to the multi-sign vault

    #[account(0, writable, signer, name="proposer", desc="Account that pays for account creation")]
    #[account(1, writable, name="stream_propsoal_account", desc="stream_propsoal_account")]
    #[account(2, name="system_program", desc="System program")]
    CreateStreamProposal = 2,

}

impl TryFrom<&u8> for MultiSignatureInstructions {
    type Error = ProgramError;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(MultiSignatureInstructions::InitMultisigVault),
            1 => Ok(MultiSignatureInstructions::AddMember),
            _ => Err(ProgramError::InvalidInstructionData)
        }
    }
}