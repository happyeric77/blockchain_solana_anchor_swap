use anchor_lang::prelude::*;
use anchor_lang::solana_program::program_option::COption;
use anchor_spl::token::{self, Mint, MintTo, TokenAccount};
// use solana_program::program::{invoke, invoke_signed};
use solana_program::pubkey::Pubkey;
// use solana_program::system_instruction;
use std::convert::TryFrom;
pub mod curve;
pub mod error;
use crate::curve::{
    base::{CurveType, SwapCurve},
    calculator::{CurveCalculator, RoundDirection, TradeDirection},
    fees::CurveFees,
};
use crate::curve::{
    constant_price::ConstantPriceCurve, constant_product::ConstantProductCurve,
    offset::OffsetCurve, stable::StableCurve,
};

declare_id!("FA1jq7srnPM7GzyXtUhXfxmTsmoaUgccWyVUWhTRH7zn");

#[program]
pub mod anchor_programs {
    use super::*;
    pub fn initialize(
        ctx: Context<Initialize>,
        fees_input: FeesInput,
        curve_input: CurveInput,
    ) -> Result<()> {
        // TODO:
        // 1. Replace the initial LP mint amt by curve calc

        // Get swap_authority address (a PDA with seed of amm account's pubkey)
        let (swap_authority, bump_seed) = Pubkey::find_program_address(
            &[&ctx.accounts.amm.to_account_info().key.to_bytes()],
            &ctx.program_id,
        );

        let _ = &ctx.accounts.validate_input_accounts(swap_authority)?;

        let curve = &ctx
            .accounts
            .validate_amm_fees_and_curve(&fees_input, &curve_input)?;

        let _ =
            &ctx.accounts
                .mint_create_state_account(bump_seed, curve_input, fees_input, curve)?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    // Swap authority: A PDA (seed: amm account's pubkey) to let program manipulate swap related features for all lp pools
    #[account(mut)]
    pub authority: AccountInfo<'info>,
    // pub amm: Box<Account<'info, Amm>>,
    #[account(mut, signer)]
    pub initializer: AccountInfo<'info>,
    #[account(init, payer=initializer, space=999)]
    pub amm: Box<Account<'info, Amm>>,

    #[account(mut)]
    pub pool_mint: Box<Account<'info, Mint>>,
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
    pub system_program: Program<'info, System>,
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
        // Fee Account & Destination Account to which The initial LP token goes MUST be owned by Authority
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

    fn validate_amm_fees_and_curve(
        &self,
        fees_input: &FeesInput,
        curve_input: &CurveInput,
    ) -> Result<(SwapCurve)> {
        let curve = build_curve(curve_input).unwrap();
        curve
            .calculator
            .validate_supply(self.token_a.amount, self.token_b.amount)?;

        let fees = build_fees(fees_input)?;
        fees.validate()?;
        curve.calculator.validate()?;
        Ok((curve))
    }

    fn mint_create_state_account(
        &mut self,
        bump_seed: u8,
        curve_input: CurveInput,
        fees_input: FeesInput,
        curve: &SwapCurve,
    ) -> Result<()> {
        // concatenate swap_authority's seed & bump
        let seeds = &[&self.amm.to_account_info().key.to_bytes(), &[bump_seed][..]];
        // Calc the inital LP token amt minted to initializer
        let initial_amount = curve.calculator.new_pool_supply();

        let mint_initial_amt_cpi_ctx = CpiContext::new(
            self.token_program.clone(),
            MintTo {
                mint: self.pool_mint.to_account_info().clone(),
                to: self.destination.to_account_info().clone(),
                authority: self.authority.clone(),
            },
        );

        token::mint_to(
            mint_initial_amt_cpi_ctx.with_signer(&[&seeds[..]]),
            u64::try_from(initial_amount).unwrap(),
        )?;

        let amm = &mut self.amm;
        amm.is_initialized = true;
        amm.bump_seed = bump_seed;
        amm.token_program_id = *self.token_program.key;
        amm.token_a_account = *self.token_a.to_account_info().key;
        amm.token_b_account = *self.token_b.to_account_info().key;
        amm.pool_mint = *self.pool_mint.to_account_info().key;
        amm.token_a_mint = self.token_a.mint;
        amm.token_b_mint = self.token_b.mint;
        amm.pool_fee_account = *self.fee_account.to_account_info().key;
        amm.fees = fees_input;
        amm.curve = curve_input;
        Ok(())
    }
}

#[account]
pub struct Amm {
    // LP creator's address
    // pub initializer_key: Pubkey,
    // pub initializer_deposit_token_account: Pubkey,
    // pub initializer_receive_token_account: Pubkey,
    // pub initializer_amount: u64,
    // pub taker_amount: u64,
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
    // Fees associated with swap
    pub fees: FeesInput,
    pub curve: CurveInput,
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

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct CurveInput {
    pub curve_type: u8,
    pub curve_parameters: u64,
}

/// Build Curve object and Fee object
pub fn build_curve(curve_input: &CurveInput) -> Result<SwapCurve> {
    let curve_type = CurveType::try_from(curve_input.curve_type).unwrap();
    let calculator: Box<dyn CurveCalculator> = match curve_type {
        CurveType::ConstantProduct => Box::new(ConstantProductCurve {}),
        CurveType::ConstantPrice => Box::new(ConstantPriceCurve {
            token_b_price: curve_input.curve_parameters,
        }),
        CurveType::Stable => Box::new(StableCurve {
            amp: curve_input.curve_parameters,
        }),
        CurveType::Offset => Box::new(OffsetCurve {
            token_b_offset: curve_input.curve_parameters,
        }),
    };
    let curve = SwapCurve {
        curve_type: curve_type,
        calculator: calculator,
    };
    Ok(curve)
}
pub fn build_fees(fees_input: &FeesInput) -> Result<CurveFees> {
    let fees = CurveFees {
        trade_fee_numerator: fees_input.trade_fee_numerator,
        trade_fee_denominator: fees_input.trade_fee_denominator,
        owner_trade_fee_numerator: fees_input.owner_trade_fee_numerator,
        owner_trade_fee_denominator: fees_input.owner_trade_fee_denominator,
        owner_withdraw_fee_numerator: fees_input.owner_withdraw_fee_numerator,
        owner_withdraw_fee_denominator: fees_input.owner_withdraw_fee_denominator,
        host_fee_numerator: fees_input.host_fee_numerator,
        host_fee_denominator: fees_input.host_fee_denominator,
    };
    Ok(fees)
}
