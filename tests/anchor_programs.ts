// import * as anchor from '@project-serum/anchor';
// import { Program } from '@project-serum/anchor';
// import { AnchorPrograms } from '../target/types/anchor_programs';
// import { Connection, PublicKey, SystemProgram, Transaction } from '@solana/web3.js';
// import { TOKEN_PROGRAM_ID, Token } from "@solana/spl-token";

// describe('anchor_programs', async() => {

//   // Configure the client to use the local cluster.
//   // anchor.setProvider(anchor.Provider.env());
//   // const provider = anchor.Provider.env();

//   const conn = new Connection("https://rpc-mainnet-fork.dappio.xyz", {
//     wsEndpoint: "wss://rpc-mainnet-fork.dappio.xyz/ws",
//     commitment: "recent",
//   });
//   const NodeWallet = require("@project-serum/anchor/src/nodewallet.js").default;
//   const wallet = NodeWallet.local();
//   const options = anchor.Provider.defaultOptions();
//   const provider = new anchor.Provider(conn, wallet, options);

//   const program = anchor.workspace.AnchorPrograms as Program<AnchorPrograms>;

//                                                                                           /**@BaseAccounts */
//   const nftCreatorAcc = anchor.web3.Keypair.generate();                                   // The nft creator state account

//   const payer = anchor.web3.Keypair.generate();                                           // payer keypair to allowcate airdropped funds
//   const initializerMainAccount = anchor.web3.Keypair.generate();                          // initializer (or main operator) account
//   let token_mint_pubkey: anchor.web3.PublicKey | undefined = undefined
//   let nft_manager: PublicKey | undefined = undefined
//   let nft_manager_seed = `nft_manager16`
//   let minted_seed: string | undefined = undefined

//   it("Setup program state", async () => {

//                                                                                   // Airdrop 1000 SOL to payer
//     await provider.connection.confirmTransaction(
//       await provider.connection.requestAirdrop(payer.publicKey, 1000000000),
//       "confirmed"
//     )
//                                                                                   // Payer funds initializer main account
//      await provider.send(                                                             // Trigger a transaction: args --> 1. Transaction 2. signer[]
//       (() => {
//         const tx = new Transaction();                                                 // Create a empty Transaction called tx (NOTE: one Transaction can contain multi instructions)
//         tx.add(                                                                       // Add first instruction into tx
//           SystemProgram.transfer({                                                    // First transaction is "SystemProgram.transfer" to fund SOL from payer to initializer's main account
//             fromPubkey: payer.publicKey,
//             toPubkey: initializerMainAccount.publicKey,
//             lamports: 900000000,
//           }),
//         );
//         return tx;
//       })(),
//       [payer]
//     );
//                                                                                                 // Check all account state
//     console.log("payer's address", payer.publicKey.toString())                                      /**@payer Address */
//     let payerBalance = await provider.connection.getBalance(payer.publicKey)
//     console.log("payer's balance: ", payerBalance/1e9, " SOL")                                        // List payer's SOL balance
//     console.log("nftCreatorAcc: ", nftCreatorAcc.publicKey.toString())                              /**@nftCreator Address */

//     let initializerBal = await provider.connection.getBalance(initializerMainAccount.publicKey)     /**@initializer Address */
//     console.log("initializer's account: ", initializerMainAccount.publicKey.toString())
//     console.log("initializer's balance: ", initializerBal/1e9, " SOL")                                // List initializer's SOL balance
//   })

//   it("is initialized", async () => {
//     let [nft_manager_pda, nft_manager_bump] = await anchor.web3.PublicKey.findProgramAddress(               // Use findProgram Address to generate PDA
//       [Buffer.from(anchor.utils.bytes.utf8.encode(nft_manager_seed))],
//       program.programId
//     )
//     const tx = await program.rpc.initialize(
//       new anchor.BN((0.3*1e9).toString()),
//       nft_manager_bump,
//       {
//         accounts: {
//           nftCreator: nftCreatorAcc.publicKey,
//           initializer: initializerMainAccount.publicKey,
//           systemProgram: anchor.web3.SystemProgram.programId,
//           nftCreaterProgram: program.programId,
//           rent: anchor.web3.SYSVAR_RENT_PUBKEY,
//           nftManager: nft_manager_pda,
//         },
//         // instructions: [await program.account.nftCreator.createInstruction(nftCreatorAcc)],
//         signers: [initializerMainAccount, nftCreatorAcc]
//       }
//     );
//     console.log("Your transaction signature", tx);
//                                                                                                       // Fetch intialized account data info
//     let data = await program.account.nftCreator.fetch(nftCreatorAcc.publicKey)
//     console.log("Created NFT items",data.collection)
//     console.log("Price: ", Number(data.price)/1e9, "SOL")
//     console.log("Total minted: ", Number(data.totalMinted))
//     nft_manager = nft_manager_pda
//   });

//   it("inititialized NFT", async () => {
//     let minter_token_acc = new anchor.web3.Keypair
//     let rand_seed = Math.round(Math.random()*10000)
//     let seed = `nft#${rand_seed}`
//     let [mint_pda, bump_seed] = await anchor.web3.PublicKey.findProgramAddress(               // Use findProgram Address to generate PDA
//         [Buffer.from(anchor.utils.bytes.utf8.encode(seed,))],
//         program.programId
//     )
//     const tx = await program.rpc.initnft(
//       bump_seed,
//       seed,
//       {                                                // Call program mintnft instruction
//         accounts: {                                                                       /**@ACCOUNTS */
//             minter: initializerMainAccount.publicKey,                                      // 1. minter as the initializer
//             nftCreater: nftCreatorAcc.publicKey,
//             nftCreaterProgram: program.programId,                                           // 2. this program id
//             mintPdaAcc: mint_pda,                                                          // 3. The mint_pda just generated
//             rent: anchor.web3.SYSVAR_RENT_PUBKEY,                                           // 4. sysVar
//             systemProgram: anchor.web3.SystemProgram.programId,
//             tokenProgram: TOKEN_PROGRAM_ID,
//         },
//         signers: [initializerMainAccount]
//     });

//     let pda_bal = await provider.connection.getBalance(mint_pda)
//     let updated_state = await program.account.nftCreator.fetch(nftCreatorAcc.publicKey)
//     console.log(
//       "\nYour transaction signature: ", tx,
//       "\nAccounts info:",
//       "\nminter: ", initializerMainAccount.publicKey.toBase58(),
//       "\nmint_pda_acc: ", mint_pda.toBase58(),
//       "\nmint_pda_lamport: ", pda_bal,
//       "\nnft_creater_program: ", program.programId.toBase58(),
//       "\nmint_seed: ", seed,
//       "\nmint_bump: ", bump_seed,
//       "\nCreated NFT items", updated_state.collection,
//       "\nTotal minted: ", Number(updated_state.totalMinted)
//     )
//     token_mint_pubkey = mint_pda
//     minted_seed = seed
//   });

//   it("mint NFT", async () => {
//     // let minter_token_acc = new anchor.web3.Keypair
//     const token_mint = new Token(
//       provider.connection,
//       token_mint_pubkey,
//       TOKEN_PROGRAM_ID,
//       initializerMainAccount // the wallet owner will pay to transfer and to create recipients associated token account if it does not yet exist.
//     );
//     const minter_ata = await token_mint.getOrCreateAssociatedAccountInfo(
//       initializerMainAccount.publicKey
//     );

//     const tx = await program.rpc.mintnft(
//       minted_seed,
//       {                                                // Call program mintnft instruction
//         accounts: {                                                                       /**@ACCOUNTS */
//             minter: initializerMainAccount.publicKey,                                          // 2. this program id
//             mintPdaAcc: token_mint.publicKey,
//             minterAta: minter_ata.address,                                                         // 3. The mint_pda just generated
//             rent: anchor.web3.SYSVAR_RENT_PUBKEY,                                           // 4. sysVar
//             nftCreator: nftCreatorAcc.publicKey,
//             nftCreatorProgram: program.programId,
//             systemProgram: anchor.web3.SystemProgram.programId,
//             tokenProgram: TOKEN_PROGRAM_ID,
//         },
//         signers: [initializerMainAccount]
//     });
//     let ata_bal = await provider.connection.getTokenAccountBalance(minter_ata.address)
//     console.log(
//       "\nYour transaction signature: ", tx,
//       "\nAccounts info:",
//       "\nMinter's token account: ", minter_ata.address.toBase58(),
//       "\nMinter's token account balance: ", ata_bal.value.amount
//     )
//   })
//   it("created metadata account", async () => {
//     let [metadata_account, metadata_account_bump] = await  PublicKey.findProgramAddress(
//       [
//         Buffer.from("metadata", "utf8"),
//         // Buffer.from("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s", "utf8"),
//         // Buffer.from(token_mint_pubkey.toBase58(), "utf8"),
//         (new PublicKey("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s")).toBytes(),
//         token_mint_pubkey.toBuffer(),
//       ],
//       new PublicKey("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s")
//     )
//     let name = "Test SOLANA NFT"
//     let symbol = "TSN"
//     let uri ="tsn.com.test"
//     const tx = await program.rpc.getmetadata(
//       metadata_account_bump,
//       name,
//       symbol,
//       uri,
//       {
//         accounts: {
//           minter: initializerMainAccount.publicKey,
//           metadataAccount: metadata_account,
//           mintPdaAcc: token_mint_pubkey,
//           nftManager: nft_manager,
//           metaplexTokenProgram: new PublicKey("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"),
//           systemProgram: anchor.web3.SystemProgram.programId,
//           rent: anchor.web3.SYSVAR_RENT_PUBKEY,
//         },
//         signers: [initializerMainAccount]
//       }
//     )
//     console.log(
//       "\nYour transaction signature: ", tx,
//       "\nMetadata account: ", metadata_account.toBase58()
//     )
//   })
// });
