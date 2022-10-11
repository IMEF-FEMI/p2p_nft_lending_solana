use anchor_lang::{prelude::*, system_program};
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Burn, CloseAccount, Mint, MintTo, SetAuthority, Token, TokenAccount, Transfer},
};
use solana_program::native_token::LAMPORTS_PER_SOL;
use spl_token::instruction::AuthorityType;

use crate::{
    errors::ErrorCode,
    math::{Decimal, TrySub},
    state::{
        loan::{Loan, LoanRequest, LoanStatus},
        PlatformFees,
    },
    utils::{
        calculate_fees, complete_loan, compound_interest, max_amount_allowed,
        uncompounded_interest, LOAN_REQUEST_STR, NFT_ESCROW_STR, PLATFORM_FEES_SEED_STR,
    },
};

pub fn request_for_loan(
    ctx: Context<LoanRequestContext>,
    nft_worth: u64,
    requested_amount: u64,
    slot_duration: u64,
) -> Result<()> {
    // couldn't use the "compound_interest" fn as it would in this case
    //exceeded maximum number of instructions allowed
    // so we use compound_interest instead
    // any difference between compounded_interest and uncompounded_interest
    // will be caught by the update loan bot
    let compounded_interest =
        uncompounded_interest(requested_amount, ctx.accounts.platform_fees.interest_rate)?;

    let max_borrow = max_amount_allowed(nft_worth, ctx.accounts.platform_fees.ltv)?;

    require!(
        compounded_interest <= max_borrow,
        ErrorCode::MaxBorrowExceeded
    );

    LoanRequest::init(
        &mut ctx.accounts.loan_request,
        nft_worth,
        ctx.accounts.nft_mint.key(),
        requested_amount,
        ctx.accounts.requested_token_mint.key(),
        slot_duration,
        ctx.accounts.borrow_nft_mint.key(),
    )?;

    let nft_amount = ctx.accounts.nft_token_account.amount;
    require!(nft_amount > 0u64, ErrorCode::InsufficientFunds);

    //transfer nft to vault
    //using platform_fees acct as the authority
    let (authority, bump) =
        Pubkey::find_program_address(&[PLATFORM_FEES_SEED_STR.as_bytes()], ctx.program_id);
    assert_eq!(authority, ctx.accounts.platform_fees.key());
    assert_eq!(
        ctx.accounts.nft_escrow.owner,
        ctx.accounts.platform_fees.key()
    );
    assert_eq!(
        ctx.accounts.nft_token_account.owner,
        ctx.accounts.borrower.key()
    );

    let bump_vecs = bump.to_le_bytes();

    let inner = vec![PLATFORM_FEES_SEED_STR.as_bytes(), bump_vecs.as_ref()];
    let outer = vec![inner.as_slice()];

    //move nft into escrow
    anchor_spl::token::transfer(
        ctx.accounts
            .transfer_into_escrow_context()
            .with_signer(outer.as_slice()),
        1u64,
    )
    .expect("transfer failed");

    //change mint authority to platform_fees
    assert_eq!(ctx.accounts.borrow_nft_mint.supply, 0);
    anchor_spl::token::set_authority(
        ctx.accounts.set_mint_authority_context(),
        AuthorityType::MintTokens,
        Some(authority),
    )
    .expect("set_authority failed");

    //mint borrow nft (an nft to represent borrower's collateral and borrowed amount)
    anchor_spl::token::mint_to(ctx.accounts.mint_context().with_signer(outer.as_slice()), 1)?;
    emit!(LoanRequestMade {
        loan_request: ctx.accounts.loan_request.key(),
        nft_worth,
        nft_mint: ctx.accounts.nft_mint.key(),
        requested_amount,
        requested_token_mint: ctx.accounts.requested_token_mint.key(),
        duration: slot_duration,
        borrow_nft_mint: ctx.accounts.borrow_nft_mint.key()
    });
    Ok(())
}

pub fn cancel_loan_request(ctx: Context<CancelRequestContext>) -> Result<()> {
    let (_authority, bump) =
        Pubkey::find_program_address(&[PLATFORM_FEES_SEED_STR.as_bytes()], ctx.program_id);

    let bump_vecs = bump.to_le_bytes();

    let inner = vec![PLATFORM_FEES_SEED_STR.as_bytes(), bump_vecs.as_ref()];
    let outer = vec![inner.as_slice()];

    match ctx.accounts.loan_request.loan {
        Some(_loan) => return Err(ErrorCode::UnableToCancel)?,
        None => {
            let nft_amount = ctx.accounts.borrow_nft_token_account.amount;
            require!(nft_amount > 0u64, ErrorCode::InsufficientFunds);

            let nft_amount = ctx.accounts.nft_escrow.amount;
            require!(nft_amount > 0u64, ErrorCode::InsufficientFunds);

            //burn borrow nft
            anchor_spl::token::burn(ctx.accounts.burn_borrow_token_context(), 1)?;
            //transfer back collateral
            anchor_spl::token::transfer(
                ctx.accounts
                    .transfer_from_escrow_context()
                    .with_signer(outer.as_slice()),
                1,
            )?;

            //close accounts
            anchor_spl::token::close_account(
                ctx.accounts
                    .close_escrow_account_context()
                    .with_signer(outer.as_slice()),
            )?;

            emit!(LoanRequestCancelled {
                loan_request: ctx.accounts.loan_request.key()
            });
        }
    }

    Ok(())
}

pub fn borrower_withdraw_tokens(ctx: Context<BorrowerWithdrawTokenContext>) -> Result<()> {
    let platform_fee = &ctx.accounts.platform_fees.key();
    let token_program = &ctx.accounts.token_program.key();
    let token_mint = &ctx.accounts.loan_request.requested_token_mint;

    let (_authority, bump) = Pubkey::find_program_address(
        &[
            platform_fee.as_ref(),
            token_program.as_ref(),
            token_mint.as_ref(),
        ],
        ctx.program_id,
    );

    let bump_vecs = bump.to_le_bytes();

    let inner = vec![
        platform_fee.as_ref(),
        token_program.as_ref(),
        token_mint.as_ref(),
        bump_vecs.as_ref(),
    ];
    let outer = vec![inner.as_slice()];

    let fee = calculate_fees(
        ctx.accounts.loan_request.requested_amount,
        ctx.accounts.platform_fees.fee_percentage,
    )
    .unwrap()
    .try_round_u64()
    .unwrap();

    let withdrawal_amount ;
    match LoanStatus::from(ctx.accounts.loan.status)? {
        LoanStatus::Started =>  withdrawal_amount = ctx.accounts.loan_request.requested_amount - fee,
        LoanStatus::Seize =>  withdrawal_amount = ctx.accounts.loan.paid_amount,
        LoanStatus::Sold => withdrawal_amount = ctx.accounts.loan.paid_amount,
        LoanStatus::Completed => withdrawal_amount = ctx.accounts.loan.paid_amount,
        _ => return Err(ErrorCode::InvalidLoanState.into()),
    }
   

    if ctx.accounts.loan_request.requested_token_mint == Pubkey::default() {
        // lamports
        system_program::transfer(
            ctx.accounts
                .transfer_lamports_from_escrow_context()
                .with_signer(outer.as_slice()),
            withdrawal_amount * LAMPORTS_PER_SOL,
        )
        .expect("transfer failed");
    } else {
        // spl_token
        anchor_spl::token::transfer(
            ctx.accounts
                .transfer_spl_tokens_from_escrow_context()
                .with_signer(outer.as_slice()),
            withdrawal_amount,
        )
        .expect("transfer failed");
    }
    ctx.accounts.loan.status = LoanStatus::TokensWithdrawn.to_code();
    Ok(())
}

pub fn repay_loan(ctx: Context<RepayLoansContext>, amount: u64) -> Result<()> {
    //can only start repaying when borrowed tokens have been taken
    require!(
        ctx.accounts.loan.status == LoanStatus::TokensWithdrawn.to_code(),
        ErrorCode::LoanDefaulted
    );

    let clock = Clock::get().unwrap();
    let current_slot = clock.slot;

    let expected_loan_end_slot =
        ctx.accounts.loan.start_slot + ctx.accounts.loan_request.slot_duration;

    let slots_elapsed = current_slot - ctx.accounts.loan.last_updated_slot;

    require!(current_slot < expected_loan_end_slot, ErrorCode::LoanEnded);

    let compounded_so_far = compound_interest(
        ctx.accounts.loan.requested_amount,
        ctx.accounts.loan.interest_rate,
        slots_elapsed,
    )?;

    let new_interest_accrued =
        compounded_so_far.try_sub(Decimal::from(ctx.accounts.loan.requested_amount))?;

    //increase outstanding_debt to capture new interest accrued
    ctx.accounts.loan.outstanding_debt += new_interest_accrued.try_round_u64()?;

    let amount_to_pay = std::cmp::min(ctx.accounts.loan.outstanding_debt, amount);

    // transfer token to escrow account (repay)
    if ctx.accounts.loan_request.requested_token_mint.key() == Pubkey::default() {
        // SOL
        system_program::transfer(
            ctx.accounts.transfer_lamports_to_escrow_context(),
            LAMPORTS_PER_SOL * amount_to_pay,
        )?;
    } else {
        //SPL - tokens
        anchor_spl::token::transfer(
            ctx.accounts.transfer_spl_tokens_to_escrow_context(),
            amount_to_pay,
        )
        .expect("transfer failed");
    }

    ctx.accounts.loan.outstanding_debt -= amount_to_pay;
    ctx.accounts.loan.paid_amount += amount_to_pay;
    ctx.accounts.loan.last_updated_slot = clock.slot;
    if ctx.accounts.loan.outstanding_debt <= 0 {
        complete_loan(ctx)?;
    }

    Ok(())
}
#[derive(Accounts)]
pub struct RepayLoansContext<'info> {
    /// CHECK: requested_token_account is the mint for the requested token Data is never read or written to
    #[account(mut)]
    pub requested_token_account: UncheckedAccount<'info>,
    #[account(mut)]
    pub loan_request: Box<Account<'info, LoanRequest>>,

    #[account(
            mut,
            seeds = [
                PLATFORM_FEES_SEED_STR.as_bytes(),
            ],
            bump,
        )]
    pub platform_fees: Box<Account<'info, PlatformFees>>,
    /// CHECK: loan_token_escrow is the mint for the requested token Data is never read or written to
    #[account(mut)]
    pub loan_token_escrow: UncheckedAccount<'info>,
    #[account(
           mut,
           constraint=loan.key() == loan_request.loan.unwrap().key()
        )]
    pub loan: Box<Account<'info, Loan>>,
    #[account(
        mut,
        constraint=borrow_nft_mint.mint_authority == platform_fees.key().into(),
    )]
    pub borrow_nft_mint: Account<'info, Mint>,
    #[account(mut)]
    pub nft_mint: Account<'info, Mint>,
    #[account(
        mut,
        associated_token::mint=nft_mint,
        associated_token::authority=borrower
    )]
    pub nft_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::mint=borrow_nft_mint,
        associated_token::authority=borrower
    )]
    pub borrow_nft_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds=[NFT_ESCROW_STR.as_bytes(), loan_request.key().as_ref()],
        bump,
        token::mint=nft_mint,
        token::authority=platform_fees,
    )]
    pub nft_escrow: Account<'info, TokenAccount>,
    #[account(mut)]
    pub borrower: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}
impl<'info> RepayLoansContext<'info> {
    pub fn burn_borrow_token_context(&self) -> CpiContext<'_, '_, '_, 'info, Burn<'info>> {
        let burn_accounts = Burn {
            from: self.borrow_nft_token_account.to_account_info().clone(),
            authority: self.borrower.to_account_info().clone(),
            mint: self.borrow_nft_mint.to_account_info().clone(),
        };
        CpiContext::new(self.token_program.to_account_info().clone(), burn_accounts)
    }
    pub fn transfer_nft_from_escrow_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let transfer_accounts = Transfer {
            from: self.nft_escrow.to_account_info().clone(),
            to: self.nft_token_account.to_account_info().clone(),
            authority: self.platform_fees.to_account_info().clone(),
        };
        CpiContext::new(
            self.token_program.to_account_info().clone(),
            transfer_accounts,
        )
    }

    pub fn transfer_spl_tokens_to_escrow_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let transfer_acct = Transfer {
            to: self.loan_token_escrow.to_account_info().clone(),
            from: self.requested_token_account.to_account_info().clone(),
            authority: self.borrower.to_account_info().clone(),
        };
        CpiContext::new(self.system_program.to_account_info(), transfer_acct)
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
}
#[derive(Accounts)]
pub struct BorrowerWithdrawTokenContext<'info> {
    /// CHECK: requested_token_account is the mint for the requested token Data is never read or written to
    #[account(mut)]
    requested_token_account: UncheckedAccount<'info>,
    #[account(mut)]
    loan_request: Box<Account<'info, LoanRequest>>,

    #[account(
        mut,
        seeds = [
            PLATFORM_FEES_SEED_STR.as_bytes(),
        ],
        bump,
    )]
    platform_fees: Box<Account<'info, PlatformFees>>,
    /// CHECK: loan_token_escrow is the mint for the requested token Data is never read or written to
    #[account(mut)]
    loan_token_escrow: UncheckedAccount<'info>,
    #[account(
       mut,
       constraint=loan.key() == loan_request.loan.unwrap().key()
    )]
    loan: Box<Account<'info, Loan>>,
    #[account(mut)]
    borrower: Signer<'info>,
    system_program: Program<'info, System>,
    token_program: Program<'info, Token>,
    associated_token_program: Program<'info, AssociatedToken>,
}
impl<'info> BorrowerWithdrawTokenContext<'info> {
    pub fn transfer_spl_tokens_from_escrow_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let transfer_acct = Transfer {
            from: self.loan_token_escrow.to_account_info().clone(),
            to: self.requested_token_account.to_account_info().clone(),
            authority: self.platform_fees.to_account_info().clone(),
        };
        CpiContext::new(self.system_program.to_account_info(), transfer_acct)
    }
    pub fn transfer_lamports_from_escrow_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, system_program::Transfer<'info>> {
        let transfer_acct = system_program::Transfer {
            from: self.loan_token_escrow.to_account_info().clone(),
            to: self.requested_token_account.to_account_info().clone(),
        };
        CpiContext::new(self.system_program.to_account_info(), transfer_acct)
    }
}

#[derive(Accounts)]
pub struct LoanRequestContext<'info> {
    #[account(mut)]
    nft_mint: Account<'info, Mint>,
    #[account(
        mut,
        associated_token::mint=nft_mint,
        associated_token::authority=borrower
    )]
    nft_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    borrow_nft_mint: Account<'info, Mint>,
    #[account(
        init,
        seeds = [LOAN_REQUEST_STR.as_bytes(), borrow_nft_mint.key().as_ref()],
        bump,
        payer = borrower,
        space = 8 + LoanRequest::MAX_SIZE,
    )]
    loan_request: Box<Account<'info, LoanRequest>>,
    #[account(
        // init,
        //transfer ownership to loan_request when its time to burn
        mut,
        associated_token::mint=borrow_nft_mint,
        associated_token::authority=borrower
    )]
    borrow_nft_token_account: Account<'info, TokenAccount>,
    /// CHECK: requested_token_mint is the mint for the requested token Data is never read or written to
    requested_token_mint: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [
            PLATFORM_FEES_SEED_STR.as_bytes(),
        ],
        bump,
    )]
    platform_fees: Box<Account<'info, PlatformFees>>,
    #[account(
        // mut,
        init,
        payer = borrower,
        seeds=[NFT_ESCROW_STR.as_bytes(), loan_request.key().as_ref()],
        bump,
        token::mint=nft_mint,
        token::authority=platform_fees,
    )]
    nft_escrow: Account<'info, TokenAccount>,
    #[account(mut)]
    borrower: Signer<'info>,
    system_program: Program<'info, System>,
    token_program: Program<'info, Token>,
    associated_token_program: Program<'info, AssociatedToken>,
    rent: Sysvar<'info, Rent>,
}

impl<'info> LoanRequestContext<'info> {
    pub fn transfer_into_escrow_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let transfer_accounts = Transfer {
            from: self.nft_token_account.to_account_info().clone(),
            to: self.nft_escrow.to_account_info().clone(),
            authority: self.borrower.to_account_info().clone(),
        };
        CpiContext::new(
            self.token_program.to_account_info().clone(),
            transfer_accounts,
        )
    }

    pub fn set_mint_authority_context(&self) -> CpiContext<'_, '_, '_, 'info, SetAuthority<'info>> {
        let cpi_accounts = SetAuthority {
            account_or_mint: self.borrow_nft_mint.to_account_info().clone(),
            current_authority: self.borrower.to_account_info().clone(),
        };
        CpiContext::new(self.token_program.to_account_info().clone(), cpi_accounts)
    }

    pub fn mint_context(&self) -> CpiContext<'_, '_, '_, 'info, MintTo<'info>> {
        let cpi_accounts = MintTo {
            mint: self.borrow_nft_mint.to_account_info().clone(),
            to: self.borrow_nft_token_account.to_account_info().clone(),
            authority: self.platform_fees.to_account_info().clone(),
        };
        CpiContext::new(self.token_program.to_account_info().clone(), cpi_accounts)
    }
}

#[derive(Accounts)]
pub struct CancelRequestContext<'info> {
    #[account(mut)]
    nft_mint: Account<'info, Mint>,
    #[account(
        mut,
        associated_token::mint=nft_mint,
        associated_token::authority=borrower
    )]
    nft_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint=borrow_nft_mint.mint_authority == platform_fees.key().into(),
    )]
    borrow_nft_mint: Account<'info, Mint>,
    #[account(
        mut,
        seeds = [LOAN_REQUEST_STR.as_bytes(), borrow_nft_mint.key().as_ref()],
        bump,
        close = borrower,
    )]
    loan_request: Box<Account<'info, LoanRequest>>,
    #[account(
        mut,
        associated_token::mint=borrow_nft_mint,
        associated_token::authority=borrower
    )]
    borrow_nft_token_account: Account<'info, TokenAccount>,
    /// CHECK: requested_token_mint is the mint for the requested token Data is never read or written to
    requested_token_mint: UncheckedAccount<'info>,

    #[account(
          mut,
          seeds = [
              PLATFORM_FEES_SEED_STR.as_bytes(),
          ],
          bump,
      )]
    platform_fees: Box<Account<'info, PlatformFees>>,
    #[account(
          mut,
          seeds=[NFT_ESCROW_STR.as_bytes(), loan_request.key().as_ref()],
          bump,
          token::mint=nft_mint,
          token::authority=platform_fees,
      )]
    nft_escrow: Account<'info, TokenAccount>,
    #[account(mut)]
    borrower: Signer<'info>,
    system_program: Program<'info, System>,
    token_program: Program<'info, Token>,
    associated_token_program: Program<'info, AssociatedToken>,
    rent: Sysvar<'info, Rent>,
}

impl<'info> CancelRequestContext<'info> {
    pub fn burn_borrow_token_context(&self) -> CpiContext<'_, '_, '_, 'info, Burn<'info>> {
        let burn_accounts = Burn {
            from: self.borrow_nft_token_account.to_account_info().clone(),
            authority: self.borrower.to_account_info().clone(),
            mint: self.borrow_nft_mint.to_account_info().clone(),
        };
        CpiContext::new(self.token_program.to_account_info().clone(), burn_accounts)
    }
    pub fn transfer_from_escrow_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let transfer_accounts = Transfer {
            from: self.nft_escrow.to_account_info().clone(),
            to: self.nft_token_account.to_account_info().clone(),
            authority: self.platform_fees.to_account_info().clone(),
        };
        CpiContext::new(
            self.token_program.to_account_info().clone(),
            transfer_accounts,
        )
    }
    pub fn close_escrow_account_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, CloseAccount<'info>> {
        let close_accounts = CloseAccount {
            account: self.nft_escrow.to_account_info().clone(),
            destination: self.borrower.to_account_info().clone(),
            authority: self.platform_fees.to_account_info().clone(),
        };
        CpiContext::new(self.token_program.to_account_info().clone(), close_accounts)
    }
}

#[event]
pub struct LoanRequestMade {
    pub loan_request: Pubkey,
    pub nft_worth: u64,
    pub nft_mint: Pubkey,
    pub requested_amount: u64,
    pub requested_token_mint: Pubkey,
    pub duration: u64,
    pub borrow_nft_mint: Pubkey,
}

#[event]
pub struct LoanRequestCancelled {
    pub loan_request: Pubkey,
}
