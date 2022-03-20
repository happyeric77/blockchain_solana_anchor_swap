use anchor_lang::prelude::*;
use anchor_lang::solana_program::program_option::COption;
use anchor_spl::token::{self, Mint, MintTo, TokenAccount, Transfer};
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
    pub fn swap(ctx: Context<Swap>, amount_in: u64, minimum_amount_out: u64) -> Result<()> {
        ctx.accounts.validate_accounts(&ctx.program_id)?;
        ctx.accounts.swap(amount_in, minimum_amount_out)?;
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

#[derive(Accounts)]
pub struct Swap<'info> {
    pub authority: AccountInfo<'info>,
    pub amm: Box<Account<'info, Amm>>,
    #[account(signer)]
    pub user_transfer_authority: AccountInfo<'info>,
    // Swapper's tokenA(orB) ATA
    #[account(mut)]
    pub source_info: AccountInfo<'info>,
    // Swapper's tokenB(orA) ATA
    #[account(mut)]
    pub destination_info: AccountInfo<'info>,
    // TokenA(orB) ata (owned by swap_authority)
    #[account(mut)]
    pub swap_source: Account<'info, TokenAccount>,
    // TokenB(orA) ata (owned by swap_authority)
    #[account(mut)]
    pub swap_destination: Account<'info, TokenAccount>,
    #[account(mut)]
    pub pool_mint: Account<'info, Mint>,
    #[account(mut)]
    pub fee_account: Account<'info, TokenAccount>,
    pub token_program: AccountInfo<'info>,
    #[account(mut)]
    pub host_fee_account: AccountInfo<'info>,
}

impl<'info> Swap<'info> {
    fn validate_accounts(&self, program_id: &Pubkey) -> Result<()> {
        let amm = &self.amm;
        if amm.to_account_info().owner != program_id {
            return Err(ProgramError::IncorrectProgramId.into());
        }
        if *self.authority.key
            != authority_id(program_id, amm.to_account_info().key, amm.bump_seed)?
        {
            return Err(error::SwapError::InvalidProgramAddress.into());
        }

        if !(*self.swap_source.to_account_info().key == amm.token_a_account
            || *self.swap_source.to_account_info().key == amm.token_b_account)
        {
            return Err(error::SwapError::IncorrectSwapAccount.into());
        }
        if !(*self.swap_destination.to_account_info().key == amm.token_a_account
            || *self.swap_destination.to_account_info().key == amm.token_b_account)
        {
            return Err(error::SwapError::IncorrectSwapAccount.into());
        }
        if *self.swap_source.to_account_info().key == *self.swap_destination.to_account_info().key {
            return Err(error::SwapError::InvalidInput.into());
        }
        if self.swap_source.to_account_info().key == self.source_info.key
            || self.swap_destination.to_account_info().key == self.destination_info.key
        {
            return Err(error::SwapError::InvalidInput.into());
        }
        if *self.pool_mint.to_account_info().key != amm.pool_mint {
            return Err(error::SwapError::IncorrectPoolMint.into());
        }
        if *self.fee_account.to_account_info().key != amm.pool_fee_account {
            return Err(error::SwapError::IncorrectFeeAccount.into());
        }
        if *self.token_program.key != amm.token_program_id {
            return Err(error::SwapError::IncorrectTokenProgramId.into());
        }
        Ok(())
    }
    fn swap(&self, amount_in: u64, minimum_amount_out: u64) -> Result<()> {
        let amm = &self.amm;
        let trade_direction = if *self.swap_source.to_account_info().key == amm.token_a_account {
            TradeDirection::AtoB
        } else {
            TradeDirection::BtoA
        };

        let curve = build_curve(&amm.curve)?;
        let fees = build_fees(&amm.fees)?;
        let result = curve
            .swap(
                u128::try_from(amount_in).unwrap(),
                u128::try_from(self.swap_source.amount).unwrap(),
                u128::try_from(self.swap_destination.amount).unwrap(),
                trade_direction,
                &fees,
            )
            .ok_or(error::SwapError::ZeroTradingTokens)?;
        if result.destination_amount_swapped < u128::try_from(minimum_amount_out).unwrap() {
            return Err(error::SwapError::ExceededSlippage.into());
        }

        let (swap_token_a_amount, swap_token_b_amount) = match trade_direction {
            TradeDirection::AtoB => (
                result.new_swap_source_amount,
                result.new_swap_destination_amount,
            ),
            TradeDirection::BtoA => (
                result.new_swap_destination_amount,
                result.new_swap_source_amount,
            ),
        };

        let seeds = &[&amm.to_account_info().key.to_bytes(), &[amm.bump_seed][..]];

        let transfer_cpi = Transfer {
            from: self.source_info.clone(),
            to: self.swap_source.to_account_info().clone(),
            authority: self.user_transfer_authority.clone(),
        };
        // Transfer Source token(AorB) amt from swapper's wallet to pool's token(AorB) ATA
        let transfer_source_amt_cpi_context =
            CpiContext::new(self.token_program.to_account_info(), transfer_cpi);

        token::transfer(
            transfer_source_amt_cpi_context.with_signer(&[&seeds[..]]),
            u64::try_from(result.source_amount_swapped).unwrap(),
        )?;

        // Handle Fees (Trade fee, Owner fee & Host fee)
        // As the transferred source amt (result.source_amount_swapped) includes all fees as the followings
        // 1. Trade fee: as source token(AorB) stright to pools Token(AorB) ATA
        // 2. Owner fee (Including Host fee): need to be converted to LP tokens and transfer to Owner (including host).
        //      So, curve.withdraw_single_token_type_exact_out is use to calc the total Owner fee as LP token value
        let total_lp_owner_fee_amt = curve
            .withdraw_single_token_type_exact_out(
                result.owner_fee, // Input the owner fee in source token value
                swap_token_a_amount,
                swap_token_b_amount,
                u128::try_from(self.pool_mint.supply).unwrap(),
                trade_direction,
                &fees,
            )
            .ok_or(error::SwapError::FeeCalculationFailure)?;
        let mut lp_owner_fee_amt: u128 = 0;
        let mut host_fee_amt: u128 = 0;

        if total_lp_owner_fee_amt > 0 {
            // If the owner fee amt >0, check if host fee lp ata is provided.
            if *self.host_fee_account.key != Pubkey::new_from_array([0; 32]) {
                // If host fee lp ata exists and this ata is assicated with lp mint,
                let host = Account::<TokenAccount>::try_from(&self.host_fee_account)?;
                if *self.pool_mint.to_account_info().key != host.mint {
                    return Err(error::SwapError::IncorrectPoolMint.into());
                }
                // then extract host fee from total owner fee amount
                host_fee_amt = fees
                    .host_fee(total_lp_owner_fee_amt)
                    .ok_or(error::SwapError::FeeCalculationFailure)?;

                if host_fee_amt > 0 {
                    // If the extracted host fee > 0, separate total owner fee to two parts
                    //  1. host_fee_amt
                    //  2. owner_fee_amt = total_lp_owner_fee_amt - host_fee_amt
                    lp_owner_fee_amt = total_lp_owner_fee_amt
                        .checked_sub(host_fee_amt)
                        .ok_or(error::SwapError::FeeCalculationFailure)?;

                    let mint_host_fee_cpi = MintTo {
                        mint: self.pool_mint.to_account_info().clone(),
                        to: self.host_fee_account.to_account_info().clone(),
                        authority: self.authority.clone(),
                    };
                    let mint_host_fee_cpi_context =
                        CpiContext::new(self.token_program.clone(), mint_host_fee_cpi);
                    // Mint host lp fee to host ata
                    token::mint_to(
                        mint_host_fee_cpi_context.with_signer(&[&seeds[..]]),
                        u64::try_from(host_fee_amt).unwrap(),
                    )?;
                } else {
                    // If If the extracted host fee = 0,
                    // owner_fee_amt equal to total_lp_owner_fee_amt
                    lp_owner_fee_amt = total_lp_owner_fee_amt;
                }
            }
            // Mint owner lp fee to fee account
            let mint_owner_fee_cpi = MintTo {
                mint: self.pool_mint.to_account_info().clone(),
                to: self.fee_account.to_account_info().clone(),
                authority: self.authority.clone(),
            };
            let mint_owner_fee_cpi_context =
                CpiContext::new(self.token_program.to_account_info(), mint_owner_fee_cpi);
            token::mint_to(
                mint_owner_fee_cpi_context.with_signer(&[&seeds[..]]),
                u64::try_from(lp_owner_fee_amt).unwrap(),
            )?;
        }
        // Transfer destination token from amm pool ata to swapper's token ata
        let transfer_dest_amt_cpi = Transfer {
            from: self.swap_destination.to_account_info(),
            to: self.destination_info.to_account_info(),
            authority: self.authority.to_account_info(),
        };
        let transfer_dest_amt_cpi_context =
            CpiContext::new(self.token_program.to_account_info(), transfer_dest_amt_cpi);
        token::transfer(
            transfer_dest_amt_cpi_context.with_signer(&[&seeds[..]]),
            u64::try_from(result.destination_amount_swapped).unwrap(),
        )?;
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

/// Calculates the authority id by generating a program address.
pub fn authority_id(program_id: &Pubkey, my_info: &Pubkey, bump_seed: u8) -> Result<Pubkey> {
    Pubkey::create_program_address(&[&my_info.to_bytes()[..32], &[bump_seed]], program_id)
        .or(Err(error::SwapError::InvalidProgramAddress.into()))
}
