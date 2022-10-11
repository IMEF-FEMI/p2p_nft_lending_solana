use std::ops::Deref;

use crate::math::{Decimal, TryDiv};
use crate::state::{ Multisig, Transaction, TransactionAccount, PlatformFees, PlatformListing, Loan, LoanFee};
use crate::utils::{MULTISIG_TX_SEED_STR, PLATFORM_FEES_SEED_STR, PLATFORM_LISTING, LOAN_FEE_STR};
use crate::{errors::ErrorCode, utils::MULTISIG_SEED_STR};
use anchor_lang::{prelude::*, system_program, };
use anchor_spl::token::{Token, Transfer};
use solana_program::instruction::Instruction;
use solana_program::native_token::LAMPORTS_PER_SOL;

///initialize multisig acct with other needed accounts
pub fn initialize_multisig(
    ctx: Context<CreateMultisig>,
    owners: Vec<Pubkey>,
    threshold: u64,
) -> Result<()> {
    let multisig = &mut ctx.accounts.multisig.clone();

    Multisig::init(&mut ctx.accounts.multisig, owners, threshold)?;
    // PlatformFees::init(&mut ctx.accounts.platform_fees, fee, interest, ltv);
    
    emit!(MultisigCreated {
        owners: multisig.owners.clone(),
        threshold: multisig.threshold,
        seqno: multisig.seqno,
    });

    Ok(())
}


// Sets the percentage fee, interest rate and ltv The only way this can be invoked
// is via a recursive call from execute_transaction -> set_owners.
pub fn set_platform_fees(
    ctx: Context<PlatformFeeAuthContext>,
    fee: u32,
    interest: u32,
    ltv: u32,) -> Result<()> {
    
    PlatformFees::init(&mut ctx.accounts.platform_fees, fee, interest, ltv);
   
  
    Ok(())
}


// Sets the owners field on the multisig. The only way this can be invoked
// is via a recursive call from execute_transaction -> set_owners.
pub fn set_owners(ctx: Context<MultisigAuth>, owners: Vec<Pubkey>) -> Result<()> {
    
    Multisig::update_owners(&mut ctx.accounts.multisig, owners.clone());
    let old_owners = owners.clone();
   
    emit!(OwnersListUpdated {
        old_owners,
        new_owners: owners
    });

    Ok(())
}
///function can be called by a multisig owner
/// to withdraw their share of the fee
pub fn withdraw_fee(
    ctx: Context<WithdrawFeeAuth>, 
) -> Result<()> {
    
// either call individually or withdraw collectively
    assert_eq!(ctx.accounts.admin.key, ctx.accounts.admin_token_account.key);
    let fee = ctx.accounts.loan_fee.amount;

    let multisig_admins = &ctx.accounts.multisig.owners;
    let loan_fee_admins = &ctx.accounts.loan_fee.owners;
    let uncollected_fees_list = &ctx.accounts.platform_fees.uncollected_fees;

    require!(uncollected_fees_list.contains(&ctx.accounts.loan_fee.key()), ErrorCode::FeeAlreadyCollected);
    require!(multisig_admins.contains(ctx.accounts.admin.key), ErrorCode::InvalidOwner);
    require!(loan_fee_admins.contains(ctx.accounts.admin.key), ErrorCode::FeeAlreadyWithdrawn);

    let multisig = &ctx.accounts.multisig.key();
    let token_program = &ctx.accounts.token_program.key();
    let token_mint = &ctx.accounts.loan_fee.token_mint.key();

    let (_authority, bump) = Pubkey::find_program_address(
        &[
            multisig.as_ref(),
            token_program.as_ref(),
            token_mint.as_ref(),
        ],
        ctx.program_id,
    );

    let bump_vecs = bump.to_le_bytes();

    let inner = vec![
        multisig.as_ref(),
        token_program.as_ref(),
        token_mint.as_ref(),
        bump_vecs.as_ref(),
    ];
    let outer = vec![inner.as_slice()];

    if ctx.accounts.loan_fee.token_mint == Pubkey::default() {
        let fee = Decimal::from(fee * LAMPORTS_PER_SOL)
        .try_div(ctx.accounts.multisig.owners.len() as u64)
        .unwrap()
        .try_floor_u64()?;
       

        system_program::transfer(
            ctx.accounts.transfer_lamports_to_admin_context().with_signer(outer.as_slice()),
            fee,
        )?;
    }else{
        
    }
// send token to accounts
    if loan_fee_admins.len() == 1 {
          
            let uncollected_fees_index = ctx
            .accounts
            .platform_fees
            .uncollected_fees
            .iter()
            .position(|a| *a == ctx.accounts.loan_fee.key()).unwrap();

            ctx
            .accounts
            .platform_fees
            .uncollected_fees.remove(uncollected_fees_index);
        }else{
            let loan_fees_index = ctx
            .accounts
            .loan_fee
            .owners
            .iter()
            .position(|a| *a == ctx.accounts.admin.key()).unwrap();

            // remove admin from loan_fee_admins
            ctx
            .accounts
            .loan_fee
            .owners.remove(loan_fees_index);
        }
    
    Ok(())
}


    // Set owners and threshold at once.
    pub fn set_owners_and_change_threshold<'info>(
        ctx: Context<'_, '_, '_, 'info, MultisigAuth<'info>>,
        owners: Vec<Pubkey>,
        threshold: u64,
    ) -> Result<()> {

       set_owners(
            Context::new(
                ctx.program_id,
                ctx.accounts,
                ctx.remaining_accounts,
                ctx.bumps.clone(),
            ),
            owners,
        )?;
        change_threshold(ctx, threshold)
    }

// change_threshold.
pub fn change_threshold(ctx: Context<MultisigAuth>, threshold: u64) -> Result<()> {
    require!(threshold > 0, ErrorCode::InvalidThreshold);

    if threshold > ctx.accounts.multisig.owners.len() as u64 {
        return Err(ErrorCode::InvalidThreshold.into());
    }
    let multisig = &mut ctx.accounts.multisig;
    multisig.threshold = threshold;
    Ok(())
}

pub fn create_transaction(
    ctx: Context<CreateTransaction>,
    program_id: Pubkey,
    transaction_accounts: Vec<TransactionAccount>,
    data: Vec<u8>,
) -> Result<()> {
    let owner_index = ctx
        .accounts
        .multisig
        .owners
        .iter()
        .position(|a| a == ctx.accounts.proposer.key)
        .ok_or(ErrorCode::InvalidOwner)?;

    let mut signers: Vec<bool> = Vec::new();
    signers.resize(ctx.accounts.multisig.owners.len(), false);
    signers[owner_index] = true;

    let tx = &mut ctx.accounts.transaction;
    tx.program_id = program_id;
    tx.accounts = transaction_accounts;
    tx.data = data;
    tx.signers = signers;
    tx.multisig = ctx.accounts.multisig.key();
    tx.did_execute = false;
    tx.seqno = ctx.accounts.multisig.seqno;
    tx.proposer = ctx.accounts.proposer.key();

    
    Ok(())
}

/// Approves a transaction on behalf of an owner of the multisig.
pub fn approve(ctx: Context<Approve>) -> Result<()> {
    let owner_index = ctx
        .accounts
        .multisig
        .owners
        .iter()
        .position(|a| a == ctx.accounts.owner.key)
        .ok_or(ErrorCode::InvalidOwner)?;

    ctx.accounts.transaction.signers[owner_index] = true;

    Ok(())
}

/// Executes the given transaction if threshold owners have signed it.
pub fn execute_transaction(ctx: Context<ExecuteTransaction>) -> Result<()> {

    if ctx.accounts.transaction.did_execute{
        return Err(ErrorCode::AlreadyExecuted.into());
    }

    //check if number of signers are up to threshold
    let sign_count = ctx
    .accounts
    .transaction
    .signers
    .iter()
    .filter( |&did_sign| *did_sign)
    .count() as u64;

    if sign_count < ctx.accounts.multisig.threshold{
        return Err(ErrorCode::NotEnoughSigners.into());
    }
    //execute 
    let mut ix: Instruction = (*ctx.accounts.transaction).deref().into();
    ix.accounts = ix
        .accounts
        .iter()
        .map(|acc| {
            let mut acc = acc.clone();
            if &acc.pubkey == ctx.accounts.multisig_signer.key {
                acc.is_signer = true;
            }
            acc
        })
        .collect();

        let (_authority, bump) = Pubkey::find_program_address(&[MULTISIG_SEED_STR.as_bytes()], ctx.program_id);

        let seeds = &[MULTISIG_SEED_STR.as_bytes(), &[bump]];
        let signer = &[&seeds[..]];
        let accounts = ctx.remaining_accounts;
        solana_program::program::invoke_signed(&ix, accounts, signer)?;

        ctx.accounts.transaction.did_execute = true;
    Ok(())
}

#[test]
pub fn test_create_multisig() {
    use std::mem;
    println!("{}", mem::size_of::<Box<Account<Multisig>>>());
}

#[derive(Accounts)]
pub struct CreateMultisig<'info> {
    #[account(
        init,
        payer = payer,
        space = 8 + Multisig::MAX_SIZE,
        seeds = [MULTISIG_SEED_STR.as_bytes()],
        bump,
    )]
    //used a pointer here to bypass the stack size limit
    // placed by anchor <Box> basically stores the data on the heap
    //reducing the stack size and allowing for more fields
    // e.x https://stackoverflow.com/questions/70747729/how-do-i-avoid-my-anchor-program-throwing-an-access-violation-in-stack-frame
    //https://stackoverflow.com/questions/70757282/stack-error-caused-working-with-anchor-on-solana
    multisig: Box<Account<'info, Multisig>>,
    #[account(
        init,
        payer = payer,
        space = 8 + PlatformFees::MAX_SIZE,
        seeds = [PLATFORM_FEES_SEED_STR.as_bytes()],
        bump,
    )]
    platform_fees: Box<Account<'info, PlatformFees>>,
    #[account(
        init,
        payer = payer,
        space = 8 + PlatformListing::MAX_SIZE,
        seeds = [PLATFORM_LISTING.as_bytes()],
        bump,
    )]
    platform_listing: Box<Account<'info, PlatformListing>>,
    #[account(mut)]
    payer: Signer<'info>,
    system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Approve<'info> {
    #[account(
        seeds = [
            MULTISIG_SEED_STR.as_bytes(),
        ],
        bump,
        constraint = multisig.seqno == transaction.seqno
    )]
    multisig: Box<Account<'info, Multisig>>,
    #[account(mut, has_one = multisig)]
    transaction: Box<Account<'info, Transaction>>,
    // One of the multisig owners. Checked in the handler.
    owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct ExecuteTransaction<'info> {
    #[account(
        seeds = [
            MULTISIG_SEED_STR.as_bytes(),
        ],
        bump,
        constraint = multisig.seqno == transaction.seqno
    )]
    multisig: Box<Account<'info, Multisig>>,
    /// CHECK: multisig_signer is a PDA program signer. Data is never read or written to
    #[account(
        seeds = [
            MULTISIG_SEED_STR.as_bytes(),
        ],
        bump,
    )]
    multisig_signer: UncheckedAccount<'info>,
    #[account(mut, 
        has_one = multisig,
        has_one = proposer,
        close = proposer,
    )]
    transaction: Box<Account<'info, Transaction>>,
    /// CHECK: multisig_signer is a PDA program signer. Data is never read or written to
    #[account(mut)]
    proposer: UncheckedAccount<'info>
}

#[derive(Accounts)]
pub struct MultisigAuth<'info> {
    #[account(
        mut,
        seeds = [
            MULTISIG_SEED_STR.as_bytes(),
        ],
        bump,
    )]
    multisig: Box<Account<'info, Multisig>>,
    #[account(
        seeds = [
            MULTISIG_SEED_STR.as_bytes(),
        ],
        bump
    )]
    multisig_signer: Signer<'info>,
}

#[derive(Accounts)]
pub struct WithdrawFeeAuth<'info> {
    #[account(
       mut,
    )]
    loan: Box<Account<'info, Loan>>,
    /// CHECK: nothing to see here 😀
    #[account(mut)]
    loan_fee_escrow: UncheckedAccount<'info>,
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
        seeds = [LOAN_FEE_STR.as_bytes(),loan.key().as_ref(),],
        bump,
    )]
    loan_fee: Box<Account<'info, LoanFee>>,
    /// CHECK: nothing to see here 😀
    #[account(mut)]
    admin_token_account: UncheckedAccount<'info>,
    #[account(
        seeds = [
            MULTISIG_SEED_STR.as_bytes(),
        ],
        bump
    )]
    multisig:Box<Account<'info, Multisig>>,
    #[account(mut)]
    admin: Signer<'info>,
    system_program: Program<'info, System>,
    token_program: Program<'info, Token>,
    rent: Sysvar<'info, Rent>,
}

impl<'info> WithdrawFeeAuth<'info> {
    pub fn transfer_spl_tokens_to_admin_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let transfer_acct = Transfer {
            from: self.loan_fee_escrow.to_account_info().clone(),
            to: self.admin_token_account.to_account_info().clone(),
            authority: self.multisig.to_account_info().clone(),
        };
        CpiContext::new(self.system_program.to_account_info(), transfer_acct)
    }
    pub fn transfer_lamports_to_admin_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, system_program::Transfer<'info>> {
        let transfer_acct = system_program::Transfer {
            from: self.loan_fee_escrow.to_account_info().clone(),
            to: self.admin_token_account.to_account_info().clone(),
        };
        CpiContext::new(self.system_program.to_account_info(), transfer_acct)
    }
}

#[derive(Accounts)]
pub struct PlatformFeeAuthContext<'info> {
    #[account(
        mut,
        seeds = [
            PLATFORM_FEES_SEED_STR.as_bytes(),
        ],
        bump,
    )]
    platform_fees: Account<'info, PlatformFees>,
    #[account(
        seeds = [MULTISIG_SEED_STR.as_bytes()],
        bump,
    )]
    multisig_signer: Signer<'info>,
}

#[derive(Accounts)]
pub struct CreateTransaction<'info> {
    #[account(
        seeds = [
            MULTISIG_SEED_STR.as_bytes(),
        ],
        bump,
    )]
    multisig: Box<Account<'info, Multisig>>,
    #[account(
        init_if_needed,
        payer = proposer,
        space = 8 + Transaction::MAX_SIZE,
        seeds = [
            MULTISIG_TX_SEED_STR.as_bytes(),
            multisig.seqno.to_le_bytes().as_ref(),
        ],
        bump
    )]
    transaction: Box<Account<'info, Transaction>>,
    #[account(mut)]
    proposer: Signer<'info>,
    system_program: Program<'info, System>,
}

//Events
#[event]
pub struct MultisigCreated {
    pub threshold: u64,
    pub seqno: u32,
    pub owners: Vec<Pubkey>,
}

#[event]
pub struct OwnersListUpdated {
    pub old_owners: Vec<Pubkey>,
    pub new_owners: Vec<Pubkey>,
}
