use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use solana_program::program::{invoke, invoke_signed};
use solana_program::pubkey::Pubkey;
use solana_program::system_instruction;
pub mod curve;
pub mod error;
use crate::curve::{
    base::SwapCurve,
    calculator::{CurveCalculator, RoundDirection, TradeDirection},
    fees::CurveFees,
};
use crate::curve::{
    constant_price::ConstantPriceCurve, constant_product::ConstantProductCurve,
    offset::OffsetCurve, stable::StableCurve,
};

declare_id!("BeJhQqHKVRtu72pnMwACnGXfqwUmEqVA777XQkWCtpgn");

#[program]
pub mod anchor_programs {
    use super::*;
    pub fn initialize(ctx: Context<Initialize>, price: u64, bump: u8) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct Initialize<'info> {
    /// CHECK: Safe
    pub authority: AccountInfo<'info>,
    #[account(signer, zero)]
    pub amm: Account<'info, Amm>,
    #[account(mut)]
    pub pool_mint: Account<'info, Mint>,
    #[account(mut)]
    pub token_a: Account<'info, TokenAccount>,
    #[account(mut)]
    pub token_b: Account<'info, TokenAccount>,
    #[account(mut)]
    pub fee_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub destination: Account<'info, TokenAccount>,
    /// CHECK: Safe
    pub token_program: AccountInfo<'info>,
}

impl<'info> Initialize<'info> {}

// #[derive(Accounts)]
// #[instruction(bump_seed: u8)]
// pub struct InitAmmPool<'info> {
//     #[account(mut, signer)]
//     pub initializer: AccountInfo<'info>,
//     #[account(init, payer=initializer)]
//     pub amm_pool: Account<'info, AmmPool>,
//     #[account(mut)]
//     pub swap_authority: AccountInfo<'info>,
//     #[account(mut)]
//     pub token_a_vault: Account<'info, TokenAccount>,
//     pub token_b_vault: Account<'info, TokenAccount>,
//     pub token_program: Program<'info, Token>,
//     pub rent: Sysvar<'info, Rent>,
// }

// impl<'info> InitNFT<'info> {
//     fn create_mint_pda_acc(&self, bump_seed: &u8, mint_seed: &String) -> ProgramResult {
//         let create_acc_ix = system_instruction::create_account(
//             // Try create account using system_instruction
//             &self.minter.key(),
//             &self.mint_pda_acc.key(),
//             self.rent.minimum_balance(Mint::LEN),
//             Mint::LEN as u64,
//             &spl_token::ID,
//         );
//         // @invoke_signed --> SYSTEM PROGRAM (bringing System Program into scope)
//         // Use invoke_signed rather than invoke -->
//         //  - THIS PROGRAM calls SYSTEM PROGRAM's create_acount instruction
//         //  - MINT_PDA_ACCOUNT calls system program to initalized itself
//         invoke_signed(
//             &create_acc_ix,
//             &[self.minter.clone(), self.mint_pda_acc.clone()],
//             // &[&[ &b"nft_creator"[..], &[bump_seed] ]]
//             // &[&[ &mint_seed.as_bytes()[..], &[*bump_seed] ]]
//             &[&[&mint_seed.as_ref(), &[*bump_seed]]],
//         )?;

//         Ok(())
//     }

//     fn init_mint_pda_acc(&self) -> ProgramResult {
//         let init_mint_ix = spl_token::instruction::initialize_mint(
//             &spl_token::ID,
//             &self.mint_pda_acc.key,
//             &self.minter.key,
//             Some(&self.minter.key),
//             0,
//         )?;
//         // @Invoke --> SPL TOKEN PROGRAM (bringing token_program into scope)
//         // Use invoke rather than invoke_sign: THIS PROGRAM calls SPL TOKEN PROGRAM's initialize_mint instruction
//         invoke(
//             &init_mint_ix,
//             &[
//                 self.minter.clone(),
//                 self.mint_pda_acc.clone(),
//                 self.rent.to_account_info().clone(),
//             ],
//         )?;
//         Ok(())
//     }

//     fn update_state(&mut self, mint_seed: &String) {
//         self.nft_creater.collection.push(mint_seed.clone());
//         self.nft_creater.total_minted += 1;
//     }
// }

// #[derive(Accounts)]
// #[instruction(seed: String)]
// pub struct MintNFT<'info> {
//     #[account(mut, signer)]
//     pub minter: AccountInfo<'info>,
//     #[account(mut)]
//     pub mint_pda_acc: Account<'info, Mint>,
//     #[account(mut)]
//     pub minter_ata: Account<'info, TokenAccount>,
//     pub nft_creator: Account<'info, NftCreator>,
//     pub nft_creator_program: AccountInfo<'info>,
//     pub system_program: Program<'info, System>,
//     pub token_program: Program<'info, Token>,
//     pub rent: Sysvar<'info, Rent>,
// }

// impl<'info> MintNFT<'info> {
//     fn mint_nft(&self) -> ProgramResult {
//         let ix = spl_token::instruction::mint_to(
//             &spl_token::ID,
//             self.mint_pda_acc.to_account_info().key,
//             self.minter_ata.to_account_info().key,
//             self.minter.key,
//             &[self.minter.key],
//             1,
//         )?;
//         invoke(
//             &ix,
//             &[
//                 self.mint_pda_acc.to_account_info().clone(),
//                 self.minter_ata.to_account_info().clone(),
//                 self.minter.clone(),
//             ],
//         )?;
//         Ok(())
//     }
// }

// #[derive(Accounts)]
// #[instruction(bump: u8, name: String, symbol: String, uri: String)]
// pub struct GetMetadata<'info> {
//     #[account(mut, signer)]
//     pub minter: AccountInfo<'info>,
//     #[account(mut)]
//     pub metadata_account: AccountInfo<'info>,
//     pub mint_pda_acc: Account<'info, Mint>,
//     pub nft_manager: AccountInfo<'info>,
//     pub metaplex_token_program: AccountInfo<'info>,
//     pub system_program: Program<'info, System>,
//     pub rent: Sysvar<'info, Rent>,
// }
// impl<'info> GetMetadata<'info> {
//     fn get_metadata(&self, bump: u8, name: String, symbol: String, uri: String) -> ProgramResult {
//         let seeds = &[
//             state::PREFIX.as_bytes(),
//             &metaplex_token_metadata::id().to_bytes(),
//             &self.mint_pda_acc.key().to_bytes(),
//         ];
//         let creator = Creator {
//             address: self.minter.key(),
//             verified: true,
//             share: 100,
//         };
//         let (metadata_account, metadata_bump) =
//             Pubkey::find_program_address(seeds, &metaplex_token_metadata::id());
//         if bump != metadata_bump {
//             return Err(NftCreatorError::IncorrectMatadataAccount.into());
//         }
//         let metadata_ix = metaplex_token_metadata::instruction::create_metadata_accounts(
//             metaplex_token_metadata::id(),
//             metadata_account.key(),
//             self.mint_pda_acc.key(),
//             self.minter.key(),
//             self.minter.key(),
//             self.minter.key(),
//             name,
//             symbol,
//             uri,
//             Some(vec![creator]),
//             0,
//             true,
//             false,
//         );
//         invoke(
//             &metadata_ix,
//             &[
//                 self.mint_pda_acc.to_account_info().clone(),
//                 self.minter.clone(),
//                 self.nft_manager.clone(),
//                 self.metadata_account.clone(),
//                 self.system_program.to_account_info().clone(),
//                 self.rent.to_account_info().clone(),
//                 self.metaplex_token_program.clone(),
//             ],
//         )?;
//         Ok(())
//     }
// }

#[account]
pub struct Amm {
    // LP creator's address
    pub initializer_key: Pubkey,
    pub initializer_deposit_token_account: Pubkey,
    pub initializer_receive_token_account: Pubkey,
    pub initializer_amount: u64,
    pub taker_amount: u64,
    /// Is the swap initialized, with data written to it
    pub is_initialized: bool,
    /// Bump seed used to generate the program address / authority
    pub bump_seed: u8,
    /// Token program ID associated with the swap
    pub token_program_id: Pubkey,
    /// Address of token A liquidity account
    pub token_a_account: Pubkey,
    /// Address of token B liquidity account
    pub token_b_account: Pubkey,
    /// Address of pool token mint
    pub pool_mint: Pubkey,
    /// Address of token A mint
    pub token_a_mint: Pubkey,
    /// Address of token B mint
    pub token_b_mint: Pubkey,
    /// Address of pool fee account
    pub pool_fee_account: Pubkey,
    /// Fees associated with swap
    pub fees: FeesInput,
    // pub curve: CurveInput,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct FeesInput {
    pub trade_fee_numerator: u64,
    pub trade_fee_denominator: u64,
    pub owner_trade_fee_numerator: u64,
    pub owner_trade_fee_denominator: u64,
    pub owner_withdraw_fee_numerator: u64,
    pub owner_withdraw_fee_denominator: u64,
    pub host_fee_numerator: u64,
    pub host_fee_denominator: u64,
}
