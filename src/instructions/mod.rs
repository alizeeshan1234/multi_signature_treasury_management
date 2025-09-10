use pinocchio::program_error::ProgramError;
use shank::ShankInstruction;

pub mod init_multisig_vault;
pub mod add_members;
pub mod create_stream_proposal;
pub mod vote_on_proposal;
pub mod deposit_funds;

#[repr(u8)]
#[derive(ShankInstruction)]
pub enum MultiSignatureInstructions {
    #[account(0, writable, signer, name="admin", desc="Account that pays for account creation")]
    #[account(1, name="mint", desc="mint account")]
    #[account(2, writable, name="multisig_info", desc="multisig_info account")]
    #[account(3, writable, name="treasury_vault_account", desc="treasury vault account")]
    #[account(4, name="token_program", desc="TokenProgram")]
    #[account(5, name="system_program", desc="System program")]
    InitMultisigVault = 0, 
    
    #[account(0, writable, signer, name="admin", desc="Account that pays for account creation")]
    #[account(1, name="member", desc="member to add")]
    #[account(2, writable, name="multisig_info", desc="multisig_info account")]
    #[account(3, name="system_program", desc="System program")]
    AddMember = 1, 

    #[account(0, writable, signer, name="proposer", desc="Account that pays for account creation")]
    #[account(1, writable, name="stream_propsoal_account", desc="stream_propsoal_account")]
    #[account(2, name = "multisig_account", desc = "multisig account that the proposal belongs to")]
    #[account(3, name="system_program", desc="System program")]
    CreateStreamProposal = 2,

    #[account(0, writable, signer, name="voter", desc="person who is voting")]
    #[account(1, writable, name="stream_proposal_account", desc="stream_propsoal_account")]
    #[account(2, writable, name="multisig_info", desc="multisig_info account")]
    #[account(3, name="system_program", desc="System program")]
    VoteOnProposal = 3,

    #[account(0, writable, signer, name="admin", desc="Account that pays for account creation")]
    #[account(1, name="mint", desc="mint account")]
    #[account(2, writable, name="source_token_account", desc="source_token_account")]
    #[account(3, writable, name="treasury_vault_account", desc="treasury vault account")]
    #[account(4, writable, name="multisig_info", desc="multisig_info account")]
    #[account(5, name="token_program", desc="Token program")]
    #[account(6, name="system_program", desc="System program")]
    DepositTokens = 4,           // Only Admin can deposit
    ExecuteApprovedProposal = 5, // Execute approved proposals
    ClaimStreamPayment = 6,      // Recipients claim their payments

    // NEW - Phase 2 (later)
    RemoveMember = 7,
    UpdateThreshold = 8,
    PauseResumeStream = 9,
    EmergencyPause = 10,
}

impl TryFrom<&u8> for MultiSignatureInstructions {
    type Error = ProgramError;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(MultiSignatureInstructions::InitMultisigVault),
            1 => Ok(MultiSignatureInstructions::AddMember),
            2 => Ok(MultiSignatureInstructions::CreateStreamProposal),
            3 => Ok(MultiSignatureInstructions::VoteOnProposal),
            4 => Ok(MultiSignatureInstructions::DepositTokens),
            5 => Ok(MultiSignatureInstructions::ExecuteApprovedProposal),
            6 => Ok(MultiSignatureInstructions::ClaimStreamPayment),
            7 => Ok(MultiSignatureInstructions::RemoveMember),
            8 => Ok(MultiSignatureInstructions::UpdateThreshold),
            9 => Ok(MultiSignatureInstructions::PauseResumeStream),
            10 => Ok(MultiSignatureInstructions::EmergencyPause),
            _ => Err(ProgramError::InvalidInstructionData)
        }
    }
}