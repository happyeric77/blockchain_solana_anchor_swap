import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
// import { AnchorPrograms } from "../target/types/anchor_programs";
import { TypeDef } from "@project-serum/anchor/dist/cjs/program/namespace/types";
import { AnchorPrograms } from "../target/types/anchor_programs";
import {
  Connection,
  PublicKey,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, Token, u64 } from "@solana/spl-token";

describe("anchor_programs", async () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());
  const provider = anchor.Provider.env();

  // const conn = new Connection("https://rpc-mainnet-fork.dappio.xyz", {
  //   wsEndpoint: "wss://rpc-mainnet-fork.dappio.xyz/ws",
  //   commitment: "recent",
  // });
  // const NodeWallet = require("@project-serum/anchor/src/nodewallet.js").default;
  // const wallet = NodeWallet.local();
  // const options = anchor.Provider.defaultOptions();
  // const provider = new anchor.Provider(conn, wallet, options);

  const program = anchor.workspace.AnchorPrograms as Program<AnchorPrograms>;

  /**@BaseAccounts */
  const ammAcc = anchor.web3.Keypair.generate(); // The nft creator state account

  const payer = anchor.web3.Keypair.generate(); // payer keypair to allowcate airdropped funds
  const initializerMainAccount = anchor.web3.Keypair.generate(); // initializer (or main operator) account
  let mintA = null as Token;
  let mintB = null as Token;
  let mintLP = null as Token;
  let tokenAata = null as PublicKey;
  let tokenBata = null as PublicKey;
  let feeAta = null as PublicKey;
  let destinationLPAccount = null;
  let [swap_authority_pda, authority_bump] =
    await anchor.web3.PublicKey.findProgramAddress(
      // Use findProgram Address to generate PDA
      [ammAcc.publicKey.toBuffer()],
      program.programId
    );

  // Pool fees
  const TRADING_FEE_NUMERATOR = 25;
  const TRADING_FEE_DENOMINATOR = 10000;
  const OWNER_TRADING_FEE_NUMERATOR = 5;
  const OWNER_TRADING_FEE_DENOMINATOR = 10000;
  const OWNER_WITHDRAW_FEE_NUMERATOR = 1;
  const OWNER_WITHDRAW_FEE_DENOMINATOR = 6;
  const HOST_FEE_NUMERATOR = 20;
  const HOST_FEE_DENOMINATOR = 100;

  // AMM curve type
  const CurveType = Object.freeze({
    ConstantProduct: 0, // Constant product curve, Uniswap-style
    ConstantPrice: 1, // Constant price curve, always X amount of A token for 1 B token, where X is defined at init
    Offset: 3, // Offset curve, like Uniswap, but with an additional offset on the token B side
  });

  it("Setup program state", async () => {
    // Airdrop 1000 SOL to payer
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(payer.publicKey, 1000000000),
      "confirmed"
    );

    // Payer funds initializer main account
    await provider.send(
      // Trigger a transaction: args --> 1. Transaction 2. signer[]
      (() => {
        const tx = new Transaction(); // Create a empty Transaction called tx (NOTE: one Transaction can contain multi instructions)
        tx.add(
          // Add first instruction into tx
          SystemProgram.transfer({
            // First transaction is "SystemProgram.transfer" to fund SOL from payer to initializer's main account
            fromPubkey: payer.publicKey,
            toPubkey: initializerMainAccount.publicKey,
            lamports: 900000000,
          })
        );
        return tx;
      })(),
      [payer]
    );
    mintA = await Token.createMint(
      provider.connection,
      payer,
      payer.publicKey,
      null,
      0,
      TOKEN_PROGRAM_ID
    );
    mintB = await Token.createMint(
      provider.connection,
      payer,
      payer.publicKey,
      null,
      0,
      TOKEN_PROGRAM_ID
    );
    mintLP = await Token.createMint(
      provider.connection,
      payer,
      swap_authority_pda,
      null,
      0,
      TOKEN_PROGRAM_ID
    );
    tokenAata = await mintA.createAccount(swap_authority_pda);
    tokenBata = await mintB.createAccount(swap_authority_pda);
    feeAta = await mintLP.createAccount(initializerMainAccount.publicKey);
    destinationLPAccount = await mintLP.createAccount(
      initializerMainAccount.publicKey
    );

    // Check all account state
    console.log(
      "payer's address",
      payer.publicKey.toString()
    ); /**@payer Address */
    let payerBalance = await provider.connection.getBalance(payer.publicKey);
    console.log("payer's balance: ", payerBalance / 1e9, " SOL"); // List payer's SOL balance
    console.log("ammAcc: ", ammAcc.publicKey.toString()); /**@amm Address */

    let initializerBal = await provider.connection.getBalance(
      initializerMainAccount.publicKey
    ); /**@initializer Address */
    console.log(
      "initializer's account: ",
      initializerMainAccount.publicKey.toString()
    );
    console.log("initializer's balance: ", initializerBal / 1e9, " SOL"); // List initializer's SOL balance
    console.log("authority pda: ", swap_authority_pda.toBase58());
    console.log("mint A pubkey: ", mintA.publicKey.toBase58());
    console.log("mint B pubkey: ", mintB.publicKey.toBase58());
    console.log("mint LP pubkey: ", mintLP.publicKey.toBase58());
    console.log("token a ata: ", tokenAata.toBase58());
    console.log("token B ata: ", tokenBata.toBase58());
    console.log("fee pubkey: ", feeAta.toBase58());
    console.log("dist ata: ", destinationLPAccount.toBase58());
  });

  it("is initialized", async () => {
    await mintA.mintTo(tokenAata, payer, [payer], 10000000);
    await mintB.mintTo(tokenBata, payer, [payer], 10000000);
    const fees_input: TypeDef<
      {
        name: "FeesInput";
        type: {
          kind: "struct";
          fields: [
            {
              name: "tradeFeeNumerator";
              type: "u64";
            },
            {
              name: "tradeFeeDenominator";
              type: "u64";
            },
            {
              name: "ownerTradeFeeNumerator";
              type: "u64";
            },
            {
              name: "ownerTradeFeeDenominator";
              type: "u64";
            },
            {
              name: "ownerWithdrawFeeNumerator";
              type: "u64";
            },
            {
              name: "ownerWithdrawFeeDenominator";
              type: "u64";
            },
            {
              name: "hostFeeNumerator";
              type: "u64";
            },
            {
              name: "hostFeeDenominator";
              type: "u64";
            }
          ];
        };
      },
      Record<string, number>
    > = {
      tradeFeeNumerator: new anchor.BN(TRADING_FEE_NUMERATOR),
      tradeFeeDenominator: new anchor.BN(TRADING_FEE_DENOMINATOR),
      ownerTradeFeeNumerator: new anchor.BN(OWNER_TRADING_FEE_NUMERATOR),
      ownerTradeFeeDenominator: new anchor.BN(OWNER_TRADING_FEE_DENOMINATOR),
      ownerWithdrawFeeNumerator: new anchor.BN(OWNER_WITHDRAW_FEE_NUMERATOR),
      ownerWithdrawFeeDenominator: new anchor.BN(
        OWNER_WITHDRAW_FEE_DENOMINATOR
      ),
      hostFeeNumerator: new anchor.BN(HOST_FEE_NUMERATOR),
      hostFeeDenominator: new anchor.BN(HOST_FEE_DENOMINATOR),
    };
    const curve_input: TypeDef<
      {
        name: "CurveInput";
        type: {
          kind: "struct";
          fields: [
            {
              name: "curveType";
              type: "u8";
            },
            {
              name: "curveParameters";
              type: "u64";
            }
          ];
        };
      },
      Record<string, number | u64>
    > = {
      curveType: CurveType.ConstantProduct,
      curveParameters: new anchor.BN(0),
    };
    const tx = await program.rpc.initialize(fees_input, curve_input, {
      accounts: {
        authority: swap_authority_pda,
        initializer: initializerMainAccount.publicKey,
        amm: ammAcc.publicKey,
        poolMint: mintLP.publicKey,
        tokenA: tokenAata,
        tokenB: tokenBata,
        feeAccount: feeAta,
        destination: destinationLPAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      },
      // instructions: [await program.account.amm.createInstruction(ammAcc)],
      signers: [ammAcc, initializerMainAccount],
      // signers: [ammAcc],
    });
    console.log("Your transaction signature", tx);
    let data = await program.account.amm.fetch(ammAcc.publicKey);
    console.log("Amm pool is initialized", data.isInitialized);
  });

  it("can swap", async () => {
    let tokenAamtToSwap = 10000;
    let minTokenBamtToGet = 1;
    // Create a new Keypair as an escrow to be delegated to transfer certain amount of source token from initializer's source token ata (in this case tokenA)
    const userTransferAuthority = anchor.web3.Keypair.generate();
    // Create an source token (tokenA) ATA for initializer
    let userAccountA = await mintA.createAccount(
      initializerMainAccount.publicKey
    );
    // Mint some source token (tokenA) ot initalizers's ATA.
    await mintA.mintTo(userAccountA, payer.publicKey, [], tokenAamtToSwap);
    // delegate userTransferAuthority to transfer some source token (tokenA) from initializer's ATA
    await mintA.approve(
      userAccountA,
      userTransferAuthority.publicKey,
      initializerMainAccount,
      [],
      tokenAamtToSwap
    );
    // Create an destination token (tokenB) ATA for initializer
    let userAccountB = await mintB.createAccount(
      initializerMainAccount.publicKey
    );
    let poolAccount = await mintLP.createAccount(payer.publicKey);
    // Trigger swap
    await program.rpc.swap(
      new anchor.BN(tokenAamtToSwap),
      new anchor.BN(minTokenBamtToGet),
      {
        accounts: {
          authority: swap_authority_pda,
          amm: ammAcc.publicKey,
          userTransferAuthority: userTransferAuthority.publicKey,
          sourceInfo: userAccountA,
          destinationInfo: userAccountB,
          swapSource: tokenAata,
          swapDestination: tokenBata,
          poolMint: mintLP.publicKey,
          feeAccount: feeAta,
          tokenProgram: TOKEN_PROGRAM_ID,
          hostFeeAccount: poolAccount,
        },
        signers: [userTransferAuthority],
      }
    );
    console.log(
      "transferAuthority acc: ",
      userTransferAuthority.publicKey.toBase58()
    );
    console.log(
      "swapper's source token - tokenA ATA: ",
      userAccountA.toBase58()
    );
    console.log(
      "swapper's destination token - tokenB ATA: ",
      userAccountB.toBase58()
    );
    console.log("host Fee Acc: ", poolAccount.toBase58());
    let userAccountAInfo = await mintA.getAccountInfo(userAccountA);
    console.log(
      "swapper's source token - tokenA balance: ",
      userAccountAInfo.amount.toNumber()
    );
    let userAccountBInfo = await mintB.getAccountInfo(userAccountB);
    console.log(
      "swapper's destination token - tokenB balance: ",
      userAccountBInfo.amount.toNumber()
    );
    let poolAccountInfo = await mintLP.getAccountInfo(poolAccount);
    console.log(
      "host fee account balance: ",
      poolAccountInfo.amount.toNumber()
    );
    let tokenAataInfo = await mintA.getAccountInfo(tokenAata);
    console.log("pool tokenA: ", tokenAataInfo.amount.toNumber());
    let tokenBataInfo = await mintB.getAccountInfo(tokenBata);
    console.log("pool tokenB: ", tokenBataInfo.amount.toNumber());
    let feeAtaInfo = await mintLP.getAccountInfo(feeAta);
    console.log("Owner fee balance", feeAtaInfo.amount.toNumber());
  });
});
