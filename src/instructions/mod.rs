use pinocchio::program_error::ProgramError;

pub mod init_multisig_vault;

#[repr(u8)]
pub enum MultiSignatureInstructions {
    InitMultisigVault = 0, //Create multi-sign vault (name, description, members, threshold)
    AddMember = 1, //Add new member to the multi-sign vault
    MultisigVaultReview = 2, //Info regarding the multi-sign vault
}

impl TryFrom<&u8> for MultiSignatureInstructions {
    type Error = ProgramError;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(MultiSignatureInstructions::InitMultisigVault),
            1 => Ok(MultiSignatureInstructions::AddMember),
            2 => Ok(MultiSignatureInstructions::MultisigVaultReview),
            _ => Err(ProgramError::InvalidInstructionData)
        }
    }
}