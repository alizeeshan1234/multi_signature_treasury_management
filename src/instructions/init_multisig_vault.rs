use pinocchio::{account_info::AccountInfo, instruction::Signer, program_error::ProgramError, pubkey::Pubkey, sysvars::{clock::Clock, rent::Rent, Sysvar}, *};
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::{state::TokenAccount, instructions::InitializeAccount3, *};

use crate::states::MultiSignatureVault;

pub fn process_init_multisig_vault(accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {

    if accounts.len() < 6 {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    let [admin, mint, multisig_info, treasury_vault, token_program, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !admin.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    };

    if instruction_data.len() < 32 {
        return Err(ProgramError::InvalidInstructionData);
    }

    let multisig_id = u64::from_le_bytes(
        instruction_data[0..8].try_into().map_err(|_| ProgramError::InvalidInstructionData)?
    );

    let threshold = u64::from_le_bytes(
        instruction_data[8..16].try_into().map_err(|_| ProgramError::InvalidInstructionData)?
    );

    let proposal_expiry = i64::from_le_bytes(
        instruction_data[16..24].try_into().map_err(|_| ProgramError::InvalidInstructionData)?
    );

    let minimum_balance = u64::from_le_bytes(
        instruction_data[24..32].try_into().map_err(|_| ProgramError::InvalidInstructionData)?
    );

    // Fixed: Validate threshold is reasonable (at least 1, max 10)
    if threshold == 0 || threshold > 10 {
        return Err(ProgramError::InvalidInstructionData);
    }

    // Fixed: Validate proposal expiry is positive (duration, not absolute time)
    if proposal_expiry <= 0 {
        return Err(ProgramError::InvalidInstructionData);
    }

    // Fixed: Validate mint is not default address
    if *mint.key() == Pubkey::default() {
        return Err(ProgramError::InvalidAccountData);
    }

    let (multisig_info_pda, multisig_info_bump) = pubkey::find_program_address(
        &[b"multisig_info", admin.key().as_ref(), multisig_id.to_le_bytes().as_ref()],
        &crate::ID
    );

    if *multisig_info.key() != multisig_info_pda {
        return Err(ProgramError::InvalidAccountData);
    };

    // Fixed: Use multisig_info_pda instead of admin.key() for consistency
    let (treasury_vault_pda, treasury_vault_bump) = pubkey::find_program_address(
        &[b"multisig_vault", mint.key().as_ref(), multisig_info_pda.as_ref()],
        &crate::ID
    );

    if *treasury_vault.key() != treasury_vault_pda {
        return Err(ProgramError::InvalidAccountData);
    };

    // Fixed: Validate token program is correct
    if *token_program.key() != pinocchio_token::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    if treasury_vault.data_is_empty() {
        let lamports = Rent::get()?.minimum_balance(TokenAccount::LEN);

        let bump_ref = &[treasury_vault_bump];
        let seeds = seeds!(
            b"multisig_vault", 
            mint.key().as_ref(), 
            multisig_info_pda.as_ref(), // Fixed: Use PDA instead of admin key
            bump_ref
        );
        let signer_seeds = Signer::from(&seeds);

        CreateAccount {
            from: admin,
            to: treasury_vault,
            lamports,
            space: TokenAccount::LEN as u64,
            owner: &pinocchio_token::id(), 
        }.invoke_signed(&[signer_seeds])?;

        InitializeAccount3 {
            account: treasury_vault,
            mint,
            owner: &multisig_info_pda, 
        }.invoke()?;
    } else {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    if multisig_info.data_is_empty() {
        let lamports = Rent::get()?.minimum_balance(MultiSignatureVault::SIZE);

        let multisig_id_ref = multisig_id.to_le_bytes();
        let bump_ref = &[multisig_info_bump];
        let seeds = seeds!(
            b"multisig_info", 
            admin.key().as_ref(), 
            multisig_id_ref.as_ref(),
            bump_ref
        );
        let signer_seeds = Signer::from(&seeds);

        CreateAccount {
            from: admin,
            to: multisig_info,
            lamports,
            space: MultiSignatureVault::SIZE as u64,
            owner: &crate::ID
        }.invoke_signed(&[signer_seeds])?;

        let mut multi_signature_vault_info = MultiSignatureVault::from_account_info_mut(multisig_info)?;
        let current_time = Clock::get()?.unix_timestamp;

        multi_signature_vault_info.id = multisig_id;
        multi_signature_vault_info.admin = *admin.key();
        multi_signature_vault_info.is_active = true;
        multi_signature_vault_info.member_count = 0;
        multi_signature_vault_info.member_keys = [Pubkey::default(); 10];
        multi_signature_vault_info.threshold = threshold;
        multi_signature_vault_info.proposal_expiry = proposal_expiry;
        multi_signature_vault_info.total_proposals = 0;
        multi_signature_vault_info.treasury_vault = *treasury_vault.key();
        multi_signature_vault_info.created_at = current_time;
        multi_signature_vault_info.last_updated = current_time; // Fixed: Use same timestamp
        multi_signature_vault_info.minimum_balance = minimum_balance;
        multi_signature_vault_info.active_proposals = 0;
        multi_signature_vault_info.executed_proposals = 0;
        multi_signature_vault_info.bump = multisig_info_bump;
        multi_signature_vault_info.treasury_vault_bump = treasury_vault_bump;
    } else {
        return Err(ProgramError::AccountAlreadyInitialized);
    };
    
    Ok(())
}
// ======================= TESTING process_init_multisig_vault =======================

// #[cfg(test)]
// mod testing {
//     use mollusk_svm::{program, Mollusk, result::Check};
//     use solana_sdk::{
//         account::Account,
//         instruction::{AccountMeta, Instruction},
//         pubkey::Pubkey,
//         program_error::ProgramError
//     };

//     const PROGRAM_ID: Pubkey = solana_sdk::pubkey!("4taWcHcTu9CbPv6JiB9HNxU9aQrni7dARoDtyv9GsWQ6");
//     const ADMIN: Pubkey = Pubkey::new_from_array([1u8; 32]);
//     const MINT: Pubkey = Pubkey::new_from_array([2u8; 32]);

//     #[test]
//     fn test_process_init_multisig_vault() {
//         let mollusk = Mollusk::new(&PROGRAM_ID, "target/deploy/multi_signature_treasury_management");

//         let multisig_id = 1u64;
//         let name = "TestVault";
//         let description = "A test multisig vault for unit testing";
//         let member_count = 3u64;
//         let threshold = 2u64;
//         let proposal_expiry = 86400i64; // 24 hours in seconds
//         let minimum_balance = 1000000u64; // 1 SOL in lamports

//         let mut instruction_data = Vec::new();
//         instruction_data.extend_from_slice(&multisig_id.to_le_bytes());
        
//         let mut name_bytes = [0u8; 12];
//         let name_str_bytes = name.as_bytes();
//         let copy_len = name_str_bytes.len().min(11); 
//         name_bytes[..copy_len].copy_from_slice(&name_str_bytes[..copy_len]);
//         instruction_data.extend_from_slice(&name_bytes);
        
//         let mut desc_bytes = [0u8; 80];
//         let desc_str_bytes = description.as_bytes();
//         let copy_len = desc_str_bytes.len().min(79); 
//         desc_bytes[..copy_len].copy_from_slice(&desc_str_bytes[..copy_len]);
//         instruction_data.extend_from_slice(&desc_bytes);
        
//         instruction_data.extend_from_slice(&member_count.to_le_bytes());
//         instruction_data.extend_from_slice(&threshold.to_le_bytes());
//         instruction_data.extend_from_slice(&proposal_expiry.to_le_bytes());
//         instruction_data.extend_from_slice(&minimum_balance.to_le_bytes());

//         let (multisig_info_pda, multisig_info_bump) = Pubkey::find_program_address(
//             &[b"multisig_info", ADMIN.as_ref(), multisig_id.to_le_bytes().as_ref()],
//             &PROGRAM_ID
//         );

//         let (treasury_vault_pda, treasury_vault_bump) = Pubkey::find_program_address(
//             &[b"multisig_vault", MINT.as_ref(), multisig_info_pda.as_ref()],
//             &PROGRAM_ID
//         );

//         let (system_program_id, system_account) = program::keyed_account_for_system_program();

//         let token_program_id = pinocchio_token::ID;
//         let token_program_account = Account {
//             lamports: 0,
//             data: vec![],
//             owner: solana_sdk::native_loader::id(),
//             executable: true,
//             rent_epoch: 0,
//         };

//         let instruction = Instruction {
//             program_id: PROGRAM_ID,
//             accounts: vec![
//                 AccountMeta::new(ADMIN, true),
//                 AccountMeta::new(MINT, false),
//                 AccountMeta::new(multisig_info_pda, false),
//                 AccountMeta::new(treasury_vault_pda, false),
//                 AccountMeta::new_readonly(system_program_id, false), 
//                 AccountMeta::new_readonly(token_program_id.into(), false),  
//             ],
//             data: instruction_data,
//         };

//         let admin_account = Account {
//             lamports: 10_000_000,
//             data: vec![],
//             owner: solana_sdk::system_program::id(),
//             executable: false,
//             rent_epoch: 0,
//         };

//         let mint_account = Account {
//             lamports: 0,
//             data: vec![],
//             owner: pinocchio_token::ID.into(),
//             executable: false,
//             rent_epoch: 0,
//         };

//         let multisig_info_account = Account {
//             lamports: 0,
//             data: vec![],
//             owner: PROGRAM_ID,
//             executable: false,
//             rent_epoch: 0,
//         };

//         let treasury_vault_account = Account {
//            lamports: 0,
//             data: vec![],
//             owner: pinocchio_token::ID.into(),
//             executable: false,
//             rent_epoch: 0,
//         };

//         mollusk.process_and_validate_instruction(
//             &instruction,
//             &vec![
//                 (ADMIN, admin_account),
//                 (MINT, mint_account),
//                 (multisig_info_pda, multisig_info_account),
//                 (treasury_vault_pda, treasury_vault_account),
//                 (token_program_id.into(), token_program_account),
//                 (system_program_id, system_account),
//             ],
//             &[Check::success()],
//         );
//     }

//     #[test]
//     fn test_process_init_multisig_vault_invalid_pda() {
//         let mollusk = Mollusk::new(&PROGRAM_ID, "target/deploy/multi_signature_treasury_management");

//         let multisig_id = 1u64;
//         let name = "TestVault";
//         let description = "A test multisig vault for unit testing";
//         let member_count = 3u64;
//         let threshold = 2u64;
//         let proposal_expiry = 86400i64; // 24 hours in seconds
//         let minimum_balance = 1000000u64; // 1 SOL in lamports

//         let mut instruction_data = Vec::new();
//         instruction_data.extend_from_slice(&multisig_id.to_le_bytes());
        
//         let mut name_bytes = [0u8; 12];
//         let name_str_bytes = name.as_bytes();
//         let copy_len = name_str_bytes.len().min(11); 
//         name_bytes[..copy_len].copy_from_slice(&name_str_bytes[..copy_len]);
//         instruction_data.extend_from_slice(&name_bytes);
        
//         let mut desc_bytes = [0u8; 80];
//         let desc_str_bytes = description.as_bytes();
//         let copy_len = desc_str_bytes.len().min(79); 
//         desc_bytes[..copy_len].copy_from_slice(&desc_str_bytes[..copy_len]);
//         instruction_data.extend_from_slice(&desc_bytes);
        
//         instruction_data.extend_from_slice(&member_count.to_le_bytes());
//         instruction_data.extend_from_slice(&threshold.to_le_bytes());
//         instruction_data.extend_from_slice(&proposal_expiry.to_le_bytes());
//         instruction_data.extend_from_slice(&minimum_balance.to_le_bytes());

//         //INVALID PDA
//         let (multisig_info_pda, multisig_info_bump) = Pubkey::find_program_address(
//             &[b"multisig", ADMIN.as_ref(), multisig_id.to_le_bytes().as_ref()],
//             &PROGRAM_ID
//         );

//         //INVALID PDA
//         let (treasury_vault_pda, treasury_vault_bump) = Pubkey::find_program_address(
//             &[b"multisig", MINT.as_ref(), multisig_info_pda.as_ref()],
//             &PROGRAM_ID
//         );

//         let (system_program_id, system_account) = program::keyed_account_for_system_program();

//         let token_program_id = pinocchio_token::ID;
//         let token_program_account = Account {
//             lamports: 0,
//             data: vec![],
//             owner: solana_sdk::native_loader::id(),
//             executable: true,
//             rent_epoch: 0,
//         };

//         let instruction = Instruction {
//             program_id: PROGRAM_ID,
//             accounts: vec![
//                 AccountMeta::new(ADMIN, true),
//                 AccountMeta::new(MINT, false),
//                 AccountMeta::new(multisig_info_pda, false),
//                 AccountMeta::new(treasury_vault_pda, false),
//                 AccountMeta::new_readonly(system_program_id, false), 
//                 AccountMeta::new_readonly(token_program_id.into(), false),  
//             ],
//             data: instruction_data,
//         };

//         let admin_account = Account {
//             lamports: 10_000_000,
//             data: vec![],
//             owner: solana_sdk::system_program::id(),
//             executable: false,
//             rent_epoch: 0,
//         };

//         let mint_account = Account {
//             lamports: 0,
//             data: vec![],
//             owner: pinocchio_token::ID.into(),
//             executable: false,
//             rent_epoch: 0,
//         };

//         let multisig_info_account = Account {
//             lamports: 0,
//             data: vec![],
//             owner: PROGRAM_ID,
//             executable: false,
//             rent_epoch: 0,
//         };

//         let treasury_vault_account = Account {
//            lamports: 0,
//             data: vec![],
//             owner: pinocchio_token::ID.into(),
//             executable: false,
//             rent_epoch: 0,
//         };

//         mollusk.process_and_validate_instruction(
//             &instruction,
//             &vec![
//                 (ADMIN, admin_account),
//                 (MINT, mint_account),
//                 (multisig_info_pda, multisig_info_account),
//                 (treasury_vault_pda, treasury_vault_account),
//                 (token_program_id.into(), token_program_account),
//                 (system_program_id, system_account),
//             ],
//             &[Check::err(ProgramError::InvalidAccountData)],
//         );
//     }

// }