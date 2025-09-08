import * as anchor from '@coral-xyz/anchor';
import { describe, it } from 'mocha';
import { expect } from 'chai';
import { Connection, PublicKey, Keypair, SystemProgram, Transaction, TransactionInstruction, SYSVAR_RENT_PUBKEY } from '@solana/web3.js';
import { AnchorProvider, Program, Wallet } from '@coral-xyz/anchor';
import fs from "fs";
import BN from "bn.js";

const idl = JSON.parse(
    fs.readFileSync("./idl/multi_signature_treasury_management.json", "utf-8")
);

if (!idl.metadata?.address) {
    throw new Error("No address found in IDL metadata");
}

const PINOCCHIO_TOKEN_PROGRAM_ID = new PublicKey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

describe("Multi Signature Vault", function() {
    const MULTISIG_ID = new BN(199);
    let connection: Connection;
    let program: Program;
    let provider: AnchorProvider;
    let multisigInfoPda: PublicKey;
    let treasuryVaultPda: PublicKey;
    let mint: PublicKey;
    let payer: Keypair;

    before(async function() {
        // FIXED: Set timeout for the before hook
        this.timeout(60000); // 60 seconds for setup

        try {
            connection = new Connection('https://api.devnet.solana.com', 'confirmed');
            
            try {
                const secretKey = JSON.parse(fs.readFileSync('./wallet.json', 'utf8'));
                payer = Keypair.fromSecretKey(Uint8Array.from(secretKey));
                console.log("Successfully loaded wallet from wallet.json");
                console.log("Wallet Public Key:", payer.publicKey.toString());
            } catch (error) {
                console.error("Error loading wallet from wallet.json:", error);
                console.log("Generating a new temporary Keypair instead.");
                payer = Keypair.generate();
                
                console.log("Requesting airdrop for generated keypair...");
                const airdropSignature = await connection.requestAirdrop(payer.publicKey, 2000000000);
                await connection.confirmTransaction(airdropSignature);
                console.log("Airdrop completed");
            }

            const wallet = new Wallet(payer);
            provider = new AnchorProvider(connection, wallet, { commitment: 'confirmed' });
            anchor.setProvider(provider);

            await createPinocchioMint();

            program = new Program(
                idl as anchor.Idl,
                idl.metadata.address, 
                provider
            );

            console.log("payer.publicKey:", provider.wallet.publicKey.toString());
            console.log("programId:", program.programId.toString());

            [multisigInfoPda] = PublicKey.findProgramAddressSync(
                [
                    Buffer.from("multisig_info"),
                    provider.wallet.publicKey.toBuffer(),
                    MULTISIG_ID.toBuffer("le", 8)
                ],
                program.programId
            );

            [treasuryVaultPda] = PublicKey.findProgramAddressSync(
                [
                    Buffer.from("multisig_vault"), 
                    mint.toBuffer(), 
                    provider.wallet.publicKey.toBuffer(),
                ],
                program.programId
            );

            console.log("Mint created:", mint.toString());
            console.log("Multisig Info PDA:", multisigInfoPda.toString());
            console.log("Treasury Vault PDA:", treasuryVaultPda.toString());

        } catch (error) {
            console.error("Error in before hook:", error);
            throw error;
        }
    });

    async function createPinocchioMint() {
        const mintKeypair = Keypair.generate();
        mint = mintKeypair.publicKey;

        const mintRentExemption = await connection.getMinimumBalanceForRentExemption(82);

        const createAccountIx = SystemProgram.createAccount({
            fromPubkey: payer.publicKey,
            newAccountPubkey: mint,
            lamports: mintRentExemption,
            space: 82, 
            programId: PINOCCHIO_TOKEN_PROGRAM_ID,
        });

        const decimals = 6;
        const mintAuthority = payer.publicKey.toBuffer();
        const freezeAuthorityOption = 0; 
        
        const initializeMintData = Buffer.alloc(67);
        initializeMintData.writeUInt8(0, 0); 
        initializeMintData.writeUInt8(decimals, 1); 
        mintAuthority.copy(initializeMintData, 2); 
        initializeMintData.writeUInt8(freezeAuthorityOption, 34); 

        const initializeMintIx = new TransactionInstruction({
            keys: [
                { pubkey: mint, isSigner: false, isWritable: true },
                { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
            ],
            programId: PINOCCHIO_TOKEN_PROGRAM_ID,
            data: initializeMintData,
        });

        const transaction = new Transaction().add(createAccountIx, initializeMintIx);
        
        try {
            const signature = await provider.sendAndConfirm(transaction, [mintKeypair], {
                commitment: 'confirmed',
                preflightCommitment: 'confirmed',
                skipPreflight: false
            });

            console.log("Mint created with Pinocchio token program:", signature);
            console.log(`View on Explorer: https://explorer.solana.com/tx/${signature}?cluster=devnet`);
        } catch (error) {
            console.error("Error creating mint:", error);
            throw error;
        }
    }

    it("Initialize multisignature vault", async function() {
        this.timeout(60000); 

        const existingMultisigInfo = await connection.getAccountInfo(multisigInfoPda);
        if (existingMultisigInfo && existingMultisigInfo.data.length > 0) {
            console.log("Multisig vault already initialized, skipping...");
            return;
        }

        const instructionDiscriminant = Buffer.from([0]); 
        
        const multisigIdBuffer = MULTISIG_ID.toBuffer("le", 8);
        
        const name = "TestVault";
        const nameBuffer = Buffer.alloc(12, 0);
        Buffer.from(name, 'utf8').copy(nameBuffer, 0);
        
        const description = "Test multisig vault for treasury management";
        const descriptionBuffer = Buffer.alloc(80, 0);
        Buffer.from(description, 'utf8').copy(descriptionBuffer, 0);
        
        const memberCount = new BN(3).toBuffer("le", 8);
        const threshold = new BN(2).toBuffer("le", 8);
        const proposalExpiry = new BN(86400).toBuffer("le", 8);
        const minimumBalance = new BN(1000000).toBuffer("le", 8);
        
        const instructionData = Buffer.concat([
            instructionDiscriminant, 
            multisigIdBuffer,    
            nameBuffer,          
            descriptionBuffer,   
            memberCount,         
            threshold,           
            proposalExpiry,      
            minimumBalance       
        ]);

        console.log("Instruction data length:", instructionData.length);
        console.log("Instruction discriminant:", instructionData[0]);
        
        const tx = new Transaction().add(
            new TransactionInstruction({
                keys: [
                    {
                        pubkey: provider.wallet.publicKey, // admin
                        isSigner: true,
                        isWritable: true,
                    },
                    {
                        pubkey: mint, // mint
                        isSigner: false,
                        isWritable: false,
                    },
                    {
                        pubkey: multisigInfoPda, // multisig_info
                        isSigner: false,
                        isWritable: true,
                    },
                    {
                        pubkey: treasuryVaultPda, // treasury_vault_account
                        isSigner: false,
                        isWritable: true,
                    },
                    {
                        pubkey: PINOCCHIO_TOKEN_PROGRAM_ID, // token_program
                        isSigner: false,
                        isWritable: false,
                    },
                    {
                        pubkey: SystemProgram.programId, // system_program
                        isSigner: false,
                        isWritable: false,
                    }
                ],
                programId: program.programId,
                data: instructionData,
            })
        );

        console.log("Sending transaction...");
        console.log("Admin:", provider.wallet.publicKey.toString());
        console.log("Mint:", mint.toString());
        console.log("Multisig Info PDA:", multisigInfoPda.toString());
        console.log("Treasury Vault PDA:", treasuryVaultPda.toString());
        console.log("Program ID:", program.programId.toString());
        console.log("Token Program:", PINOCCHIO_TOKEN_PROGRAM_ID.toString());

        try {
            const signature = await provider.sendAndConfirm(tx, [], {
                commitment: 'confirmed',
                preflightCommitment: 'confirmed',
                skipPreflight: false
            });
            
            console.log("Transaction signature:", signature);
            console.log(`View on Solana Explorer: https://explorer.solana.com/tx/${signature}?cluster=devnet`);

            await new Promise(resolve => setTimeout(resolve, 2000));

            const multisigInfo = await connection.getAccountInfo(multisigInfoPda);
            const treasuryVault = await connection.getAccountInfo(treasuryVaultPda);
            
            console.log("Multisig Info Account created:", multisigInfo !== null);
            console.log("Treasury Vault Account created:", treasuryVault !== null);
            
            if (multisigInfo) {
                console.log("Multisig Info data length:", multisigInfo.data.length);
            }
            if (treasuryVault) {
                console.log("Treasury Vault data length:", treasuryVault.data.length);
            }
            expect(multisigInfo).to.not.be.null;
            expect(treasuryVault).to.not.be.null;
            
        } catch (error) {
            console.error("Transaction failed:", error);
            throw error;
        }
    });
});