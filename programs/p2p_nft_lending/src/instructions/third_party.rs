use crate::errors::ErrorCode;
use crate::math::{Decimal, TrySub};
use crate::state::{Loan, LoanStatus, PlatformFees};
use crate::utils::{compound_interest, PLATFORM_FEES_SEED_STR};
use anchor_lang::{prelude::*, system_program};
use anchor_spl::token::{Mint, Token, TokenAccount, Transfer};
use solana_program::native_token::LAMPORTS_PER_SOL;

pub fn refresh_loan(ctx: Context<RefreshContext>) -> Result<()> {
    let current_status = LoanStatus::from(ctx.accounts.loan.status)?;
    require!(
        current_status.to_code() == LoanStatus::TokensWithdrawn.to_code(),
        ErrorCode::CantRefreshLoan
    );

    let clock = Clock::get().unwrap();
    let current_slot = clock.slot;

    let expected_loan_end_slot = ctx.accounts.loan.start_slot + ctx.accounts.loan.slot_duration;

    let slots_elapsed = current_slot - ctx.accounts.loan.last_updated_slot;

    let compounded_so_far = compound_interest(
        ctx.accounts.loan.requested_amount,
        ctx.accounts.loan.interest_rate,
        slots_elapsed,
    )?;
    let new_interest_accrued =
        compounded_so_far.try_sub(Decimal::from(ctx.accounts.loan.requested_amount))?;

    ctx.accounts.loan.outstanding_debt += new_interest_accrued.try_round_u64()?;

    // change state to Defaulted if expired
    if current_slot > expected_loan_end_slot {
        ctx.accounts.loan.status = LoanStatus::Defaulted.to_code();
    }

    ctx.accounts.loan.last_updated_slot = clock.slot;

    Ok(())
}

pub fn buy_nft(ctx: Context<BuyNftContext>) -> Result<()> {
    require!(
        ctx.accounts.loan.status == LoanStatus::Sell.to_code(),
        ErrorCode::InvalidLoanState
    );

    // transfer tokens to escrow
    if ctx.accounts.loan.requested_token_mint == Pubkey::default() {
        system_program::transfer(
            ctx.accounts.transfer_lamports_to_escrow_context(),
            (ctx.accounts.loan.nft_worth) * LAMPORTS_PER_SOL,
        )
        .expect("transfer failed");
    } else {
        anchor_spl::token::transfer(
            ctx.accounts.transfer_spl_tokens_to_escrow_context(),
            ctx.accounts.loan.nft_worth,
        )
        .expect("transfer failed");
    }

    // transfer nft to buyer
    let (_authority, bump) =
        Pubkey::find_program_address(&[PLATFORM_FEES_SEED_STR.as_bytes()], ctx.program_id);

    let bump_vecs = bump.to_le_bytes();

    let inner = vec![PLATFORM_FEES_SEED_STR.as_bytes(), bump_vecs.as_ref()];
    let outer = vec![inner.as_slice()];
    anchor_spl::token::transfer(
        ctx.accounts
            .transfer_nft_from_escrow_context()
            .with_signer(outer.as_slice()),
        1,
    )?;
    // change state
    ctx.accounts.loan.status = LoanStatus::Sold.to_code();

    Ok(())
}

#[derive(Accounts)]
pub struct BuyNftContext<'info> {
    pub nft_mint: Account<'info, Mint>,
    #[account(
        mut,
        associated_token::mint=nft_mint,
        associated_token::authority=buyer
    )]
    pub buyer_nft_account: Account<'info, TokenAccount>,
    /// CHECK
    #[account(
        constraint=requested_token_mint.key() == loan.requested_token_mint.key()
    )]
    requested_token_mint: UncheckedAccount<'info>, //could be Pubkey::Default()
    /// CHECK: requested_token_account is the mint for the requested token Data is never read or written to
    #[account(mut)]
    requested_token_account: UncheckedAccount<'info>, //could be Pubkey::Default()
    #[account(
        mut,
        seeds = [
            PLATFORM_FEES_SEED_STR.as_bytes(),
        ],
        bump,
    )]
    platform_fees: Box<Account<'info, PlatformFees>>,
    /// CHECK
    #[account(mut)]
    loan_token_escrow: UncheckedAccount<'info>,
    #[account(
        mut,
        token::mint=nft_mint,
        token::authority=platform_fees,
    )]
    pub nft_escrow: Account<'info, TokenAccount>,
    #[account(mut)]
    pub loan: Box<Account<'info, Loan>>,
    #[account(mut)]
    pub buyer: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

impl<'info> BuyNftContext<'info> {
    pub fn transfer_nft_from_escrow_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let transfer_accounts = Transfer {
            from: self.nft_escrow.to_account_info().clone(),
            to: self.buyer_nft_account.to_account_info().clone(),
            authority: self.platform_fees.to_account_info().clone(),
        };
        CpiContext::new(
            self.token_program.to_account_info().clone(),
            transfer_accounts,
        )
    }

    pub fn transfer_lamports_to_escrow_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, system_program::Transfer<'info>> {
        let transfer_acct = system_program::Transfer {
            from: self.requested_token_account.to_account_info().clone(),
            to: self.loan_token_escrow.to_account_info().clone(),
        };
        CpiContext::new(self.system_program.to_account_info(), transfer_acct)
    }
    pub fn transfer_spl_tokens_to_escrow_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let transfer_acct = Transfer {
            from: self.requested_token_account.to_account_info().clone(),
            to: self.loan_token_escrow.to_account_info().clone(),
            authority: self.buyer.to_account_info().clone(),
        };
        CpiContext::new(self.system_program.to_account_info(), transfer_acct)
    }
}
#[derive(Accounts)]
pub struct RefreshContext<'info> {
    #[account(mut)]
    pub loan: Box<Account<'info, Loan>>,
}
