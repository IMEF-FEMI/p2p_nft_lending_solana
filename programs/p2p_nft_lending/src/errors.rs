use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Not enough owners signed this transaction.")]
    NotEnoughSigners,
    #[msg("Overflow when adding.")]
    Overflow,
    #[msg("Owners must be unique")]
    UniqueOwners,   
    #[msg("The given transaction has already been executed.")]
    AlreadyExecuted,
    #[msg("Threshold must be less than or equal to the number of owners.")]
    InvalidThreshold,
    #[msg("Multisig has already been created.")]
    MultisigAlreadyInitialized,
    #[msg("The given owner is not part of this multisig.")]
    InvalidOwner,
    #[msg("Owners length must be non zero.")]
    InvalidOwnersLen,
    #[msg("Math operation overflow")]
    MathOverflow,
    #[msg("Insufficient funds")]
    InsufficientFunds,
    #[msg("Unable to cancel loan Request")]
    UnableToCancel,
    #[msg("Maximum borrow amount exceeded")]
    MaxBorrowExceeded,
    #[msg("Account provided is not correct")]
    InvalidAccount,
    #[msg("Wrong Loan Status")]
    InvalidStatus,
    #[msg("Loan has ended")]
    LoanEnded,
    #[msg("Loan has ended and the borrower has defaulted")]
    LoanDefaulted,
    #[msg("Unable to refresh loan at this time")]
    CantRefreshLoan,
    #[msg("Uncollected List is Full")]
    FeesListFull,
    #[msg("Admin(owner) has already withdrawn allocated fees")]
    FeeAlreadyWithdrawn,
    #[msg("fees collected already ")]
    FeeAlreadyCollected,
    #[msg("Unable to perform action at this time")]
    InvalidLoanState,
}
impl From<ErrorCode> for ProgramError {
    fn from(e: ErrorCode) -> Self {
        ProgramError::Custom(e as u32)
    }
}