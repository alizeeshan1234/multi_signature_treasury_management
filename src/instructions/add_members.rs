use pinocchio::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey, sysvars::{clock::Clock, rent::Rent, Sysvar}, *};
use pinocchio_log::log;

use crate::states::MultiSignatureVault;

pub fn process_add_member(accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {

    if accounts.len() < 4 {
        return Err(ProgramError::InvalidAccountData);
    };

    let [admin, member, multisig_info, system_program] = accounts else {
        return Err(ProgramError::InvalidAccountData);
    };

    if !admin.is_signer() {
        return Err(ProgramError::InvalidAccountData);
    };

    let multisig_id = u64::from_le_bytes(
        instruction_data[0..8].try_into().map_err(|_| ProgramError::InvalidInstructionData)?
    );

    let (multisig_info_pda, multisig_info_bump) = pubkey::find_program_address(
        &[b"multisig_info", admin.key().as_ref(), multisig_id.to_le_bytes().as_ref()],
        &crate::ID
    );

    if *multisig_info.key() != multisig_info_pda {
        return Err(ProgramError::InvalidAccountData);
    };

    let mut multisig_account_info = MultiSignatureVault::from_account_info_mut(multisig_info)?;

    if multisig_account_info.admin != *admin.key() {
        log!("Only the vault admin can add members");
        return Err(ProgramError::InvalidAccountData);
    }

    if !multisig_account_info.is_active {
        log!("Cannot add member: vault is inactive");
        return Err(ProgramError::InvalidAccountData);
    }

    for i in 0..10 {
        if multisig_account_info.member_keys[i] == *member.key() {
            log!("Member already exists in the member list!");
            return Err(ProgramError::InvalidAccountData);
        }
    }

    let mut member_added = false;
    for i in 0..10 {
        if multisig_account_info.member_keys[i] == Pubkey::default() {
            multisig_account_info.member_keys[i] = *member.key();
            member_added = true;
            log!("Member added successfully at index {}", i);
            break;
        }
    }

    if !member_added {
        log!("Cannot add member: maximum capacity (10) reached");
        return Err(ProgramError::InvalidAccountData);
    }

    multisig_account_info.member_count = multisig_account_info.member_count.checked_add(1).unwrap();
    multisig_account_info.last_updated = Clock::get()?.unix_timestamp;
    log!("Member added successfully!");

    Ok(())
}

// ======================= TESTING process_add_member =======================
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use mollusk_svm::{program, Mollusk, result::Check};
//     use solana_sdk::{
//         account::Account,
//         instruction::{AccountMeta, Instruction},
//         pubkey::Pubkey,
//         program_error::ProgramError,
//         pubkey,
//     };
//     use crate::states::MultiSignatureVault;

//     const PROGRAM_ID: Pubkey = pubkey!("4taWcHcTu9CbPv6JiB9HNxU9aQrni7dARoDtyv9GsWQ6");
//     const ADMIN: Pubkey = Pubkey::new_from_array([1u8; 32]);
//     const NEW_MEMBER: Pubkey = Pubkey::new_from_array([2u8; 32]);

//     #[test]
//     fn test_process_add_member_success() {
//         let mollusk = Mollusk::new(&PROGRAM_ID, "target/deploy/multi_signature_treasury_management");
//         let multisig_id = 1u64;

//         let mut instruction_data = Vec::new();
//         instruction_data.extend_from_slice(&multisig_id.to_le_bytes());

//         let (multisig_info_pda, multisig_info_bump) = Pubkey::find_program_address(
//             &[b"multisig_info", ADMIN.as_ref(), multisig_id.to_le_bytes().as_ref()],
//             &PROGRAM_ID
//         );

//         let (system_program_id, system_account) = program::keyed_account_for_system_program();

//         let mut vault_data = vec![0u8; MultiSignatureVault::SIZE];
//         let vault = MultiSignatureVault {
//             id: multisig_id,
//             admin: ADMIN.to_bytes(),
//             is_active: true,
//             name: [0u8; 12],
//             description: [0u8; 80],
//             member_count: 0,
//             member_keys: [[0u8; 32]; 10],
//             threshold: 2,
//             proposal_expiry: 86400,
//             total_proposals: 0,
//             treasury_vault: Pubkey::new_unique().to_bytes(),
//             created_at: 1640995200,
//             last_updated: 1640995200,
//             minimum_balance: 1_000_000,
//             active_proposals: 0,
//             executed_proposals: 0,
//             bump: multisig_info_bump,
//             treasury_vault_bump: 255,
//         };

//         unsafe {
//             let vault_ptr = &vault as *const MultiSignatureVault as *const u8;
//             std::ptr::copy_nonoverlapping(vault_ptr, vault_data.as_mut_ptr(), MultiSignatureVault::SIZE);
//         }

//         let instruction = Instruction {
//             program_id: PROGRAM_ID,
//             accounts: vec![
//                 AccountMeta::new(ADMIN, true),
//                 AccountMeta::new(NEW_MEMBER, false),
//                 AccountMeta::new(multisig_info_pda, false),
//                 AccountMeta::new_readonly(system_program_id, false),
//             ],
//             data: instruction_data
//         };

//         let admin_account = Account {
//             lamports: 10_000_000,
//             data: vec![],
//             owner: solana_sdk::system_program::id(),
//             executable: false,
//             rent_epoch: 0,
//         };

//         let new_member_account = Account {
//             lamports: 10_000_000,
//             data: vec![],
//             owner: solana_sdk::system_program::id(),
//             executable: false,
//             rent_epoch: 0,
//         };

//         let multisig_info_account = Account {
//             lamports: 1_000_000,
//             data: vault_data,
//             owner: PROGRAM_ID,
//             executable: false,
//             rent_epoch: 0,
//         };

//         mollusk.process_and_validate_instruction(
//             &instruction,
//             &vec![
//                 (ADMIN, admin_account),
//                 (NEW_MEMBER, new_member_account),
//                 (multisig_info_pda, multisig_info_account),
//                 (system_program_id, system_account),
//             ],
//             &[Check::success()]
//         );
//     }
// }