import * as anchor from '@coral-xyz/anchor';
import { describe, it } from 'mocha';
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
    const MULTISIG_ID = new BN(206);
    const PROPOSAL_ID = new BN(11);
    let connection: Connection;
    let program: Program;
    let provider: AnchorProvider;
    let multisigInfoPda: PublicKey;
    let treasuryVaultPda: PublicKey;
    let mint: PublicKey;
    let payer: Keypair;
    let streamProposalAccount: PublicKey;

    before(async function() {
        this.timeout(60000); 

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

            [streamProposalAccount] = PublicKey.findProgramAddressSync(
                [
                    Buffer.from("stream_proposal"),
                    PROPOSAL_ID.toBuffer("le", 8),
                    MULTISIG_ID.toBuffer("le", 8)
                ],
                program.programId
            )

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

        const multisigIdBuffer = MULTISIG_ID.toBuffer("le", 8);     // 8 bytes (0-7)
        const threshold = new BN(2).toBuffer("le", 8);             // 8 bytes (8-15)  
        const proposalExpiry = new BN(86400).toBuffer("le", 8);    // 8 bytes (16-23)
        const minimumBalance = new BN(1000000).toBuffer("le", 8);  // 8 bytes (24-31)
    
        const instructionData = Buffer.concat([
            multisigIdBuffer,     // bytes 0-7
            threshold,           // bytes 8-15
            proposalExpiry,      // bytes 16-23  
            minimumBalance       // bytes 24-31
        ]);

        console.log("Instruction data length:", instructionData.length);
        console.log("Expected length: 32 bytes");
    
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
                data: Buffer.concat([
                    Buffer.from([0]), // instruction discriminant (InitMultisigVault = 0)
                    instructionData   // actual instruction data
                ])
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

        } catch (error) {
            console.error("Transaction failed:", error);
            if (error.logs) {
                console.error("Program logs:");
                error.logs.forEach((log, i) => console.error(`  ${i}: ${log}`));
            }
            throw error;
        }
    });

    it("Add member", async function() {
        this.timeout(60000); 

        const existingMultisigInfo = await connection.getAccountInfo(multisigInfoPda);
        if (!existingMultisigInfo || existingMultisigInfo.data.length === 0) {
            console.log("Multisig vault not initialized, skipping add member test...");
            return;
        }

        const instructionDiscriminant = Buffer.from([1]); 
        const multisigIdBuffer = MULTISIG_ID.toBuffer("le", 8);
        const instructionData = Buffer.concat([instructionDiscriminant, multisigIdBuffer]);

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
                        pubkey: provider.wallet.publicKey, // member to be added
                        isSigner: false,
                        isWritable: true,
                    },
                    {
                        pubkey: multisigInfoPda, // multisig_info
                        isSigner: false,
                        isWritable: true,
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

    console.log("Sending Add Member Transaction...");
    console.log("Admin:", provider.wallet.publicKey.toString());
    console.log("Member to be added:", provider.wallet.publicKey.toString());
    console.log("Multisig Info PDA:", multisigInfoPda.toString());
    console.log("Program ID:", program.programId.toString());

    try {
        const signature = await provider.sendAndConfirm(tx, [], {
            commitment: 'confirmed',
            preflightCommitment: 'confirmed',
            skipPreflight: false
        });

        console.log(`Transaction Signature: ${signature}`);
        console.log(`View on Solana Explorer: https://explorer.solana.com/tx/${signature}?cluster=devnet`);
    } catch (error) {
        console.error("Add member transaction failed:", error);
        if (error.logs) {
            console.error("Program logs:");
            error.logs.forEach((log, i) => console.error(`  ${i}: ${log}`));
        }
        throw error;
    }
});

    it("Create Stream Proposal", async function() {
        this.timeout(60000); 

        const instructionDiscriminant = Buffer.from([2]); 
        const proposalIdBuffer = PROPOSAL_ID.toBuffer("le", 8); 
        const multisigIdBuffer = MULTISIG_ID.toBuffer("le", 8); 
        const streamType = Buffer.from([1]); // Fixed: Convert BN to single byte buffer
        const requiredThreshold = Buffer.from([2]); // Fixed: Convert BN to single byte buffer             
        const votingDeadline = new BN(86400).toBuffer("le", 8);    

        const streamNameStr = "Treasury Stream 001";
        const streamNameBuffer = Buffer.alloc(32);
        Buffer.from(streamNameStr, 'utf8').copy(streamNameBuffer, 0);

        const streamDescStr = "Monthly treasury distribution stream for community rewards and development funding";
        const streamDescBuffer = Buffer.alloc(128);
        Buffer.from(streamDescStr, 'utf8').copy(streamDescBuffer, 0);

        const instructionData = Buffer.concat([
            instructionDiscriminant,  // 1 byte
            proposalIdBuffer,         // 8 bytes (0-8)
            multisigIdBuffer,         // 8 bytes (8-16) 
            streamType,               // 1 byte (16) - Fixed
            requiredThreshold,        // 1 byte (17) - Fixed
            votingDeadline,           // 8 bytes (18-26)
            streamNameBuffer,         // 32 bytes (26-58)
            streamDescBuffer          // 128 bytes (58-186)
        ]);

        console.log("Instruction data length:", instructionData.length); // Should be 186 bytes

        const tx = new Transaction().add(
            new TransactionInstruction({
                keys: [
                    {
                        pubkey: provider.wallet.publicKey, // proposer
                        isSigner: true,
                        isWritable: true,
                    },
                    {
                        pubkey: streamProposalAccount, // stream_proposal_account
                        isSigner: false,
                        isWritable: true,
                    },
                    {
                        pubkey: multisigInfoPda, // multisig_info
                        isSigner: false,
                        isWritable: true,
                    },
                    {
                        pubkey: SystemProgram.programId, // system_program
                        isSigner: false,
                        isWritable: false,
                    }
                ],
                programId: program.programId,
                data: instructionData
            })
        );

        console.log(`Stream Proposal Account PDA: ${streamProposalAccount.toString()}`);

        const signature = await provider.sendAndConfirm(tx, [], {
            commitment: 'confirmed',
            preflightCommitment: 'confirmed',
            skipPreflight: false
        });
    
        console.log("Transaction signature:", signature);
        console.log(`View on Solana Explorer: https://explorer.solana.com/tx/${signature}?cluster=devnet`);
    });

    it("Vote On Proposal", async function () {
        this.timeout(60000); 
        const VOTE_TYPE = new BN(0);

        const instructionDiscriminant = Buffer.from([3]); 
        const proposalIdBuffer = PROPOSAL_ID.toBuffer("le", 8); 
        const multisigIdBuffer = MULTISIG_ID.toBuffer("le", 8); 
        const voteType = VOTE_TYPE.toBuffer("le", 8);

        const instructionData = Buffer.concat([
            instructionDiscriminant,
            proposalIdBuffer,
            multisigIdBuffer,
            voteType
        ]);

        console.log("Instruction data length:", instructionData.length); 
        console.log(`Instruction Data: ${instructionData}`);

        const tx = new Transaction().add(
            new TransactionInstruction({
                keys: [
                    {
                        pubkey: provider.wallet.publicKey, // proposer
                        isSigner: true,
                        isWritable: true,
                    },
                    {
                        pubkey: streamProposalAccount, // stream_proposal_account
                        isSigner: false,
                        isWritable: true,
                    },
                    {
                        pubkey: multisigInfoPda, // multisig_info
                        isSigner: false,
                        isWritable: true,
                    },
                    {
                        pubkey: SystemProgram.programId, // system_program
                        isSigner: false,
                        isWritable: false,
                    }
                ],
                programId: program.programId,
                data: instructionData
            })
        );

        const signature = await provider.sendAndConfirm(tx, [], {
            commitment: 'confirmed',
            preflightCommitment: 'confirmed',
            skipPreflight: false
        });

        console.log("Transaction signature:", signature);
        console.log(`View on Solana Explorer: https://explorer.solana.com/tx/${signature}?cluster=devnet`);
    })
});
