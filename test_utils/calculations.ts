import * as anchor from "@project-serum/anchor";
import { AnchorProvider } from "@project-serum/anchor";

//constants
export const DEFAULT_TICKS_PER_SECOND = 160;
export const DEFAULT_TICKS_PER_SLOT = 64;
export const SECONDS_PER_DAY = 24 * 60 * 60;


/// Number of slots per year
export const SLOTS_PER_YEAR =
    DEFAULT_TICKS_PER_SECOND / DEFAULT_TICKS_PER_SLOT * SECONDS_PER_DAY * 365;

export const slotsInDuration = (seconds: number) => DEFAULT_TICKS_PER_SECOND / DEFAULT_TICKS_PER_SLOT * seconds;
export const slotsInAYear = () => slotsInDuration(SECONDS_PER_DAY * 365)
/**
 * value borrowAmount will be compounded to by the end of the loan (duration)
 * @param borrowAmount amount to borrow 
 * @param interestRate current interest rate
 * @param duration loan duration 
 * @returns number
 */
export const compoundInterest = (
    borrowAmount: number,
    interestRate: number,
    duration: number,
): number => {
    //
    const actualInterestRate = interestRate / 1000;
    const slotElapsed = slotsInDuration(duration);
    const slotInterestRate = actualInterestRate / SLOTS_PER_YEAR;

    const compoundedInterestRate = (1 + slotInterestRate) ** slotElapsed;

    return compoundedInterestRate * borrowAmount;

}

/**
 * maximum amount that can be borrowed against the collateral
 * @param nftWorth worth of the NFT
 * @param ltv current platform ltv (collateral factor)
 */
export const maxAllowedAmount = (
    nftWorth: number,
    ltv: number,
): number => {
    const actualLtv = ltv / 1000;
    return nftWorth * actualLtv;
}
export const calculateFees = (
    amount: number,
    feePercentage: number,
): number => {
    const actualFeePercentage = feePercentage / 1000;
    return amount * actualFeePercentage
}