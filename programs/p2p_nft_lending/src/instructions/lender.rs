use anchor_lang::{prelude::*, system_program};
use anchor_spl::{
    associated_token::{AssociatedToken, Create},
    token::{Burn, Mint, MintTo, SetAuthority, Token, TokenAccount, Transfer},
};
use solana_program::native_token::LAMPORTS_PER_SOL;
use spl_token::instruction::AuthorityType;

use crate::{
    errors::ErrorCode,
    state::{
        loan::{GrantLoan, Loan, LoanRequest},
        LoanFee, LoanStatus, Multisig, PlatformFees,
    },
    utils::{
        calculate_fees, GRANT_LOAN_STR, LOAN_FEE_STR, LOAN_STR, MULTISIG_SEED_STR, NFT_ESCROW_STR,
        PLATFORM_FEES_SEED_STR,
    },
};

pub fn grant_loan(ctx: Context<GrantLoanContext>) -> Result<()> {
    let loan_token_escrow_bump = ctx.bumps.get(PLATFORM_FEES_SEED_STR);
    let bump = &[*loan_token_escrow_bump.unwrap()][..];
    let inner = vec![PLATFORM_FEES_SEED_STR.as_bytes(), bump];
    let outer = vec![inner.as_slice()];
    let clock = Clock::get().unwrap();

    let fee = calculate_fees(
        ctx.accounts.loan_request.requested_amount,
        ctx.accounts.platform_fees.fee_percentage,
    )
    .unwrap()
    .try_round_u64()
    .unwrap();

    let remainder = ctx.accounts.loan_request.requested_amount - fee;
    assert_eq!(fee + remainder, ctx.accounts.loan_request.requested_amount);
    // checks that could not be done using anchor constraints
    assert!(fee > 0);
    if ctx.accounts.requested_token_mint.key() == Pubkey::default() {
        //we're using lamports
        require!(
            ctx.accounts.requested_token_account.key() == ctx.accounts.lender.key(),
            ErrorCode::InvalidAccount
        );

        system_program::transfer(
            ctx.accounts.transfer_lamports_to_fee_escrow_context(),
            LAMPORTS_PER_SOL * fee,
        )?;
        system_program::transfer(
            ctx.accounts.transfer_lamports_to_escrow_context(),
            LAMPORTS_PER_SOL * remainder,
        )?;
    } else {
        //we're using spl token

        // create loan and loan fee escrow accts
        require!(
            ctx.accounts.requested_token_account.owner.key() == ctx.accounts.lender.key(),
            ErrorCode::InvalidAccount
        );

        anchor_spl::associated_token::create(
            ctx.accounts
                .create_loan_token_escrow_context()
                .with_signer(outer.as_slice()),
        )?;
        anchor_spl::associated_token::create(
            ctx.accounts
                .create_loan_fee_token_escrow_context()
                .with_signer(outer.as_slice()),
        )?;

        // complete transfer
        anchor_spl::token::transfer(
            ctx.accounts
                .transfer_spl_tokens_to_loan_fee_escrow_context()
                .with_signer(outer.as_slice()),
            fee,
        )
        .expect("transfer failed");
        anchor_spl::token::transfer(
            ctx.accounts
                .transfer_spl_tokens_to_escrow_context()
                .with_signer(outer.as_slice()),
            remainder,
        )
        .expect("transfer failed");
    }

    //grant program authority to mint
    anchor_spl::token::set_authority(
        ctx.accounts.set_mint_authority_context(),
        AuthorityType::MintTokens,
        Some(ctx.accounts.platform_fees.key()),
    )
    .expect("set_authority failed");

    //mint lend nft (an nft to represent lender's collateral and amount given out as loan)
    anchor_spl::token::mint_to(
        ctx.accounts
            .mint_loan_nft_context()
            .with_signer(outer.as_slice()),
        1,
    )?;

    GrantLoan::init(
        &mut ctx.accounts.grant_loan_req,
        ctx.accounts.loan_request.nft_worth,
        ctx.accounts.loan_request.requested_amount,
        ctx.accounts.requested_token_mint.key(),
        ctx.accounts.loan_request.key(),
        ctx.accounts.loan_request.slot_duration,
        ctx.accounts.lend_nft_mint.key(),
    )?;
    //init loan
    Loan::init(
        &mut ctx.accounts.loan,
        ctx.accounts.loan_request.nft_mint.key(),
        ctx.accounts.loan_request.borrow_nft_mint.key(),
        ctx.accounts.lend_nft_mint.key(),
        ctx.accounts.requested_token_mint.key(),
        ctx.accounts.platform_fees.ltv,
        ctx.accounts.platform_fees.fee_percentage,
        ctx.accounts.platform_fees.interest_rate,
        ctx.accounts.loan_request.nft_worth,
        ctx.accounts.loan_request.requested_amount,
        ctx.accounts.loan_request.slot_duration,
        clock.slot,
    )?;
    //init loan fee
    LoanFee::init(
        &mut ctx.accounts.loan_fee,
        fee,
        ctx.accounts.requested_token_mint.key(),
        ctx.accounts.loan.key(),
        ctx.accounts.loan_fee_escrow.key(),
        ctx.accounts.multisig.owners.clone(),
    );
    ctx.accounts.loan_request.loan = Some(ctx.accounts.loan.key());

    if ctx.accounts.platform_fees.uncollected_fees.len() == 50 {
        return Err(ErrorCode::FeesListFull.into());
    }
    ctx.accounts
        .platform_fees
        .uncollected_fees
        .push(ctx.accounts.loan_fee.key());

    emit!(LoanGranted {
        nft_mint: ctx.accounts.loan_request.nft_mint,
        loan_request: ctx.accounts.loan_request.key(),
        requested_amount: ctx.accounts.loan_request.requested_amount,
        requested_token_mint: ctx.accounts.requested_token_mint.key(),
        duration: ctx.accounts.loan_request.slot_duration,
        lend_nft_mint: ctx.accounts.lend_nft_mint.key()
    });

    Ok(())
}

pub fn lender_withdraw_tokens(ctx: Context<LenderWithdrawTokenContext>) -> Result<()> {


    let platform_fee = &ctx.accounts.platform_fees.key();
    let token_program = &ctx.accounts.token_program.key();
    let token_mint = &ctx.accounts.grant_loan_req.requested_token_mint;

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

    let withdrawal_amount ;
    match LoanStatus::from(ctx.accounts.loan.status)? {
        LoanStatus::Repaid =>  withdrawal_amount = ctx.accounts.loan.paid_amount,
        LoanStatus::Sold => withdrawal_amount = ctx.accounts.loan.nft_worth,
        _ => return Err(ErrorCode::InvalidLoanState.into()),
    }
    if ctx.accounts.grant_loan_req.requested_token_mint == Pubkey::default() {
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
    anchor_spl::token::burn(ctx.accounts.burn_lend_nft_context(), 1)?;

    ctx.accounts.loan.status = LoanStatus::Completed.to_code();

    Ok(())
}

pub fn seize_nft(ctx: Context<SeizeNftContext>) -> Result<()> {
    require!(
        LoanStatus::from(ctx.accounts.loan.status)
            .unwrap()
            .to_code()
            == LoanStatus::Defaulted.to_code(),
        ErrorCode::InvalidLoanState
    );
    let (_authority, bump) =
        Pubkey::find_program_address(&[PLATFORM_FEES_SEED_STR.as_bytes()], ctx.program_id);

    let bump_vecs = bump.to_le_bytes();

    let inner = vec![PLATFORM_FEES_SEED_STR.as_bytes(), bump_vecs.as_ref()];
    let outer = vec![inner.as_slice()];

    // transfer nft to lender
    anchor_spl::token::transfer(
        ctx.accounts
            .transfer_nft_from_escrow_context()
            .with_signer(outer.as_slice()),
        1,
    )?;

    //burn lend nft
    anchor_spl::token::burn(ctx.accounts.burn_lend_token_context(), 1)?;

    //set state to Seize
    ctx.accounts.loan.status = LoanStatus::Seize.to_code();
    Ok(())
}

pub fn sell_nft(ctx: Context<SellNftContext>) -> Result<()> {
    require!(
        LoanStatus::from(ctx.accounts.loan.status)
            .unwrap()
            .to_code()
            == LoanStatus::Defaulted.to_code(),
        ErrorCode::InvalidLoanState
    );
    ctx.accounts.loan.status = LoanStatus::Sell.to_code();
    Ok(())
}

#[derive(Accounts)]
pub struct SellNftContext<'info> {
    #[account(
        constraint=loan.lend_nft_mint == lend_nft_mint.key()
    )]
    lend_nft_mint: Account<'info, Mint>,
    #[account(
        mut,
        associated_token::mint=lend_nft_mint,
        associated_token::authority=lender,
        // close = lender
    )]
    lend_nft_account: Account<'info, TokenAccount>,
    #[account(mut)]
    loan: Box<Account<'info, Loan>>,
    #[account(mut)]
    pub lender: Signer<'info>,
}
#[derive(Accounts)]
pub struct SeizeNftContext<'info> {
    pub nft_mint: Account<'info, Mint>,
    #[account(
        mut,
        associated_token::mint=nft_mint,
        associated_token::authority=lender
    )]
    pub lender_nft_account: Account<'info, TokenAccount>,
    #[account(mut)]
    lend_nft_mint: Account<'info, Mint>,
    #[account(
        mut,
        associated_token::mint=lend_nft_mint,
        associated_token::authority=lender,
        // close = lender
    )]
    lend_nft_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [
            PLATFORM_FEES_SEED_STR.as_bytes(),
        ],
        bump,
    )]
    pub platform_fees: Box<Account<'info, PlatformFees>>,
    #[account(
        mut,
        seeds=[NFT_ESCROW_STR.as_bytes(), grant_loan_req.loan_request.key().as_ref()],
        bump,
        token::mint=nft_mint,
        token::authority=platform_fees,
    )]
    pub nft_escrow: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [GRANT_LOAN_STR.as_bytes(), lend_nft_mint.key().as_ref()],
        bump,
        close = lender
    )]
    grant_loan_req: Box<Account<'info, GrantLoan>>,

    #[account(mut)]
    loan: Box<Account<'info, Loan>>,
    #[account(mut)]
    pub lender: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

impl<'info> SeizeNftContext<'info> {
    pub fn burn_lend_token_context(&self) -> CpiContext<'_, '_, '_, 'info, Burn<'info>> {
        let burn_accounts = Burn {
            from: self.lend_nft_account.to_account_info().clone(),
            authority: self.lender.to_account_info().clone(),
            mint: self.lend_nft_mint.to_account_info().clone(),
        };
        CpiContext::new(self.token_program.to_account_info().clone(), burn_accounts)
    }
    pub fn transfer_nft_from_escrow_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let transfer_accounts = Transfer {
            from: self.nft_escrow.to_account_info().clone(),
            to: self.lender_nft_account.to_account_info().clone(),
            authority: self.platform_fees.to_account_info().clone(),
        };
        CpiContext::new(
            self.token_program.to_account_info().clone(),
            transfer_accounts,
        )
    }
}
#[derive(Accounts)]
pub struct LenderWithdrawTokenContext<'info> {
    #[account(mut)]
    lend_nft_mint: Account<'info, Mint>,
    #[account(
        mut,
        associated_token::mint=lend_nft_mint,
        associated_token::authority=lender
    )]
    lend_nft_account: Account<'info, TokenAccount>,
    /// CHECK: requested_token_mint is the mint for the requested token Data is never read or written to
    #[account(
        constraint=requested_token_mint.key() == grant_loan_req.requested_token_mint.key()
    )]
    requested_token_mint: UncheckedAccount<'info>, //could be Pubkey::Default()
    /// CHECK: requested_token_account is the mint for the requested token Data is never read or written to
    #[account(mut)]
    requested_token_account: UncheckedAccount<'info>, //could be Pubkey::Default()
    #[account(
        mut,
        seeds = [GRANT_LOAN_STR.as_bytes(), lend_nft_mint.key().as_ref()],
        bump,
        // close = lender
    )]
    grant_loan_req: Box<Account<'info, GrantLoan>>,
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
        seeds = [LOAN_STR.as_bytes(),grant_loan_req.loan_request.key().as_ref(), grant_loan_req.key().as_ref()],
        bump,
        // close = lender
    )]
    loan: Box<Account<'info, Loan>>,
    #[account(
        seeds = [MULTISIG_SEED_STR.as_bytes()],
        bump,
    )]
    multisig: Box<Account<'info, Multisig>>,
    /// CHECK: loan_token_escrow is the mint for the requested token Data is never read or written to
    #[account(mut)]
    loan_token_escrow: UncheckedAccount<'info>,
    #[account(mut)]
    lender: Signer<'info>,
    system_program: Program<'info, System>,
    token_program: Program<'info, Token>,
    associated_token_program: Program<'info, AssociatedToken>,
    rent: Sysvar<'info, Rent>,
}

impl<'info> LenderWithdrawTokenContext<'info> {
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
    pub fn burn_lend_nft_context(&self) -> CpiContext<'_, '_, '_, 'info, Burn<'info>> {
        let burn_accounts = Burn {
            from: self.lend_nft_account.to_account_info().clone(),
            authority: self.lender.to_account_info().clone(),
            mint: self.lend_nft_mint.to_account_info().clone(),
        };
        CpiContext::new(self.token_program.to_account_info().clone(), burn_accounts)
    }
}
#[derive(Accounts)]
pub struct GrantLoanContext<'info> {
    #[account(mut)]
    lend_nft_mint: Account<'info, Mint>,
    #[account(
        mut,
        associated_token::mint=lend_nft_mint,
        associated_token::authority=lender
    )]
    lend_nft_account: Account<'info, TokenAccount>,
    /// CHECK: requested_token_mint is the mint for the requested token Data is never read or written to
    #[account(
        constraint=requested_token_mint.key() == loan_request.requested_token_mint.key()
    )]
    requested_token_mint: UncheckedAccount<'info>, //could be Pubkey::Default()
    /// CHECK: requested_token_account is the mint for the requested token Data is never read or written to
    #[account(mut)]
    requested_token_account: UncheckedAccount<'info>, //could be Pubkey::Default()
    // #[account(
    #[account(mut)]
    loan_request: Box<Account<'info, LoanRequest>>,
    #[account(
        init,
        payer = lender,
        space = 8 + GrantLoan::MAX_SIZE,
        seeds = [GRANT_LOAN_STR.as_bytes(), lend_nft_mint.key().as_ref()],
        bump,
    )]
    grant_loan_req: Box<Account<'info, GrantLoan>>,
    #[account(
        mut,
        seeds = [
            PLATFORM_FEES_SEED_STR.as_bytes(),
        ],
        bump,
    )]
    platform_fees: Box<Account<'info, PlatformFees>>,
    #[account(
        init,
        payer = lender,
        space = 8 + Loan::MAX_SIZE,
        seeds = [LOAN_STR.as_bytes(),loan_request.key().as_ref(), grant_loan_req.key().as_ref()],
        bump,
    )]
    loan: Box<Account<'info, Loan>>,
    #[account(
        init,
        payer = lender,
        space = 8 + LoanFee::MAX_SIZE,
        seeds = [LOAN_FEE_STR.as_bytes(),loan.key().as_ref(),],
        bump,
    )]
    loan_fee: Box<Account<'info, LoanFee>>,
    /// CHECK: loan_fee_escrow is the fee escrow for the requested token
    #[account(mut)]
    loan_fee_escrow: UncheckedAccount<'info>,
    #[account(
        seeds = [MULTISIG_SEED_STR.as_bytes()],
        bump,
    )]
    multisig: Box<Account<'info, Multisig>>,
    /// CHECK: loan_token_escrow is the mint for the requested token Data is never read or written to
    #[account(mut)]
    loan_token_escrow: UncheckedAccount<'info>,
    #[account(mut)]
    lender: Signer<'info>,
    system_program: Program<'info, System>,
    token_program: Program<'info, Token>,
    associated_token_program: Program<'info, AssociatedToken>,
    rent: Sysvar<'info, Rent>,
}

impl<'info> GrantLoanContext<'info> {
    pub fn create_loan_fee_token_escrow_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, Create<'info>> {
        let create_loan_fee_escrow_accounts = Create {
            payer: self.lender.to_account_info().clone(),
            associated_token: self.loan_fee_escrow.to_account_info().clone(),
            authority: self.multisig.to_account_info().clone(),
            mint: self.requested_token_mint.to_account_info().clone(),
            system_program: self.system_program.to_account_info().clone(),
            token_program: self.token_program.to_account_info().clone(),
            rent: self.rent.to_account_info().clone(),
        };

        CpiContext::new(
            self.associated_token_program.to_account_info().clone(),
            create_loan_fee_escrow_accounts,
        )
    }
    pub fn create_loan_token_escrow_context(&self) -> CpiContext<'_, '_, '_, 'info, Create<'info>> {
        let create_loan_escrow_account = Create {
            payer: self.lender.to_account_info().clone(),
            associated_token: self.loan_token_escrow.to_account_info().clone(),
            authority: self.platform_fees.to_account_info().clone(),
            mint: self.requested_token_mint.to_account_info().clone(),
            system_program: self.system_program.to_account_info().clone(),
            token_program: self.token_program.to_account_info().clone(),
            rent: self.rent.to_account_info().clone(),
        };

        CpiContext::new(
            self.associated_token_program.to_account_info().clone(),
            create_loan_escrow_account,
        )
    }
    pub fn transfer_spl_tokens_to_escrow_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let transfer_acct = Transfer {
            from: self.requested_token_account.to_account_info().clone(),
            to: self.loan_token_escrow.to_account_info().clone(),
            authority: self.lender.to_account_info().clone(),
        };
        CpiContext::new(self.system_program.to_account_info(), transfer_acct)
    }
    pub fn transfer_spl_tokens_to_loan_fee_escrow_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let transfer_acct = Transfer {
            from: self.requested_token_account.to_account_info().clone(),
            to: self.loan_fee_escrow.to_account_info().clone(),
            authority: self.lender.to_account_info().clone(),
        };
        CpiContext::new(self.system_program.to_account_info(), transfer_acct)
    }
    pub fn transfer_lamports_to_escrow_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, system_program::Transfer<'info>> {
        let transfer_acct = system_program::Transfer {
            from: self.lender.to_account_info().clone(),
            to: self.loan_token_escrow.to_account_info().clone(),
        };
        CpiContext::new(self.system_program.to_account_info(), transfer_acct)
    }
    pub fn transfer_lamports_to_fee_escrow_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, system_program::Transfer<'info>> {
        let transfer_acct = system_program::Transfer {
            from: self.lender.to_account_info().clone(),
            to: self.loan_fee_escrow.to_account_info().clone(),
        };
        CpiContext::new(self.system_program.to_account_info(), transfer_acct)
    }
    pub fn set_mint_authority_context(&self) -> CpiContext<'_, '_, '_, 'info, SetAuthority<'info>> {
        let cpi_accounts = SetAuthority {
            account_or_mint: self.lend_nft_mint.to_account_info().clone(),
            current_authority: self.lender.to_account_info().clone(),
        };
        CpiContext::new(self.token_program.to_account_info().clone(), cpi_accounts)
    }
    pub fn mint_loan_nft_context(&self) -> CpiContext<'_, '_, '_, 'info, MintTo<'info>> {
        let cpi_accounts = MintTo {
            mint: self.lend_nft_mint.to_account_info().clone(),
            to: self.lend_nft_account.to_account_info().clone(),
            authority: self.platform_fees.to_account_info().clone(),
        };
        CpiContext::new(self.token_program.to_account_info().clone(), cpi_accounts)
    }
}

#[event]
pub struct LoanGranted {
    pub nft_mint: Pubkey,
    pub loan_request: Pubkey,
    pub requested_amount: u64,
    pub requested_token_mint: Pubkey,
    pub duration: u64,
    pub lend_nft_mint: Pubkey,
}
