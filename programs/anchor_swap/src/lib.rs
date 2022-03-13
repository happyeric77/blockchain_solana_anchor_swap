use anchor_lang::prelude::*;
use anchor_lang::solana_program::program_option::COption;
use anchor_spl::token::{self, Mint, MintTo, TokenAccount};
// use solana_program::program::{invoke, invoke_signed};
use solana_program::pubkey::Pubkey;
// use solana_program::system_instruction;
use std::convert::TryFrom;
pub mod curve;
pub mod error;
// use crate::curve::{
//     base::SwapCurve,
//     calculator::{CurveCalculator, RoundDirection, TradeDirection},
//     fees::CurveFees,
// };
// use crate::curve::{
//     constant_price::ConstantPriceCurve, constant_product::ConstantProductCurve,
//     offset::OffsetCurve, stable::StableCurve,
// };

declare_id!("BeJhQqHKVRtu72pnMwACnGXfqwUmEqVA777XQkWCtpgn");

#[program]
pub mod anchor_programs {
    use super::*;
    pub fn initialize(
        ctx: Context<Initialize>,
        // fees_input: FeesInput,
        // curve_input: CurveInput,
    ) -> Result<()> {
        // TODO:
        // 1. Replace the initial LP mint amt by curve calc

        // Get swap_authority address (a PDA with seed of amm account's pubkey)
        let (swap_authority, bump_seed) = Pubkey::find_program_address(
            &[&ctx.accounts.amm.to_account_info().key.to_bytes()],
            &ctx.program_id,
        );

        let _ = &ctx.accounts.validate_input_accounts(swap_authority)?;

        // concatenate swap_authority's seed & bump
        let seeds = &[
            &ctx.accounts.amm.to_account_info().key.to_bytes(),
            &[bump_seed][..],
        ];

        // calc initial LP mint amt
        let initial_amount = 1 as u128;

        let mint_initial_amt_cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.clone(),
            MintTo {
                mint: ctx.accounts.pool_mint.to_account_info().clone(),
                to: ctx.accounts.destination.to_account_info().clone(),
                authority: ctx.accounts.authority.clone(),
            },
        );

        token::mint_to(
            mint_initial_amt_cpi_ctx.with_signer(&[&seeds[..]]),
            u64::try_from(initial_amount).unwrap(),
        )?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    // Swap authority: A PDA (seed: amm account's pubkey) to let program manipulate swap related features for all lp pools
    pub authority: AccountInfo<'info>,
    #[account(signer, zero)]
    pub amm: Account<'info, Amm>,
    #[account(mut)]
    pub pool_mint: Account<'info, Mint>,
    // amm's token A account
    #[account(mut)]
    pub token_a: Account<'info, TokenAccount>,
    // amm's token B account
    #[account(mut)]
    pub token_b: Account<'info, TokenAccount>,
    #[account(mut)]
    pub fee_account: Account<'info, TokenAccount>,
    // The LP token ATA to which the initial LP token is sent (Owner MUST be authority)
    #[account(mut)]
    pub destination: Account<'info, TokenAccount>,
    pub token_program: AccountInfo<'info>,
}

impl<'info> Initialize<'info> {
    fn validate_input_accounts(&self, swap_authority: Pubkey) -> Result<()> {
        // TODO:
        // 1. use input curve_input to create curve object
        // 2. Add Swap constraint
        if self.amm.is_initialized {
            return Err(error::SwapError::AlreadyInUse.into());
        }
        // Verify if input authority pubkey is valid
        if *self.authority.key != swap_authority {
            return Err(error::SwapError::InvalidProgramAddress.into());
        }
        if *self.authority.key != self.token_a.owner || *self.authority.key != self.token_b.owner {
            return Err(error::SwapError::InvalidOwner.into());
        }
        // TODO: What is destination??
        if *self.authority.key == self.fee_account.owner
            && *self.authority.key == self.destination.owner
        {
            return Err(error::SwapError::InvalidOutputOwner.into());
        }
        if COption::Some(*self.authority.key) != self.pool_mint.mint_authority {
            return Err(error::SwapError::InvalidOwner.into());
        }
        if self.token_a.mint == self.token_b.mint {
            return Err(error::SwapError::RepeatedMint.into());
        }
        // Amm's A token accounts MUST NOT have any delegation
        if self.token_a.delegate.is_some() || self.token_b.delegate.is_some() {
            return Err(error::SwapError::InvalidDelegate.into());
        }
        // Amm's B token accounts MUST NOT have Close Authority
        if self.token_a.close_authority.is_some() || self.token_b.close_authority.is_some() {
            return Err(error::SwapError::InvalidCloseAuthority.into());
        }
        // Amm's LP mint supply MUST be 0
        if self.pool_mint.supply != 0 {
            return Err(error::SwapError::InvalidSupply.into());
        }
        // Amm's LP mint MUST NOT have Freeze Authority
        if self.pool_mint.freeze_authority.is_some() {
            return Err(error::SwapError::InvalidFreezeAuthority.into());
        }
        // Amm's LP mint pubkey MUST be == input Fee Account's mint
        if *self.pool_mint.to_account_info().key != self.fee_account.mint {
            return Err(error::SwapError::IncorrectPoolMint.into());
        }
        Ok(())
    }
}

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
