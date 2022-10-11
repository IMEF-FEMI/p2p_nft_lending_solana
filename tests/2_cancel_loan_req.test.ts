import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { LAMPORTS_PER_SOL, } from "@solana/web3.js";
import assert from "assert";
import { P2pNftLending } from "../target/types/p2p_nft_lending";
import { maxAllowedAmount, compoundInterest, slotsInAYear, calculateFees } from "../test_utils/calculations";
import {
    GRANT_LOAN_STR,
    LOAN_FEE_STR,
    LOAN_REQUEST_STR,
    LOAN_STR,
    MULTISIG_SEED_STR,
    NFT_ESCROW_STR,
    PLATFORM_FEES_SEED_STR,
    PLATFORM_LISTING_STR
} from "../test_utils/CONSTANTS";
import { sleep } from "../test_utils/generalUtils";
import { PDAParameters } from "../test_utils/types";
import {
    createAssociatedTokenAccount,
    createAssociatedTokenAccountAndMintTo,
    createMint,
    findAssociatedTokenAddress,
    getKeypair,
    getLoanPDA,
    getMultisigTransactionPdaParams,
    getPdaParams,
    getPdaParamsWithSeedAndPubkey,
    mintTokens,
    tokenAccountInfo,
    writePublicKey,
} from "../test_utils/walletUtils";


let owner1: anchor.web3.Keypair;
let owner2: anchor.web3.Keypair;
let owner3: anchor.web3.Keypair;
let owner4: anchor.web3.Keypair;
let owner5: anchor.web3.Keypair;
let newOwners: anchor.web3.PublicKey[];



// Borrower
let nftMint: anchor.web3.PublicKey;
let borrower: anchor.web3.Keypair;
let borrowerMainNftAccount: anchor.web3.PublicKey;
let borrowNftMint: anchor.web3.PublicKey;
let borrowNftAccount: anchor.web3.PublicKey;
let borrowerTokenAccount: anchor.web3.PublicKey;

// Lender
let lender: anchor.web3.Keypair;
let requestedTokenMint: anchor.web3.PublicKey;
let lenderTokenAccount: anchor.web3.PublicKey;
let lendNftMint: anchor.web3.PublicKey;
let lendNftAccount: anchor.web3.PublicKey;


// PDAs
let platformFees: PDAParameters;
let platformListing: PDAParameters;
let multisigPda: PDAParameters;
let loanRequest: PDAParameters;
let grantLoan: PDAParameters;
let loan: anchor.web3.PublicKey;
let loanFee: anchor.web3.PublicKey;

//escrow
let nftEscrowTokenAccount: PDAParameters;
let escrowTokenAccount: anchor.web3.PublicKey;
let feeEscrowTokenAccount: anchor.web3.PublicKey;

const threshold = new anchor.BN(2);
//fees
const fee = new anchor.BN(3 * 10);
const ltv = new anchor.BN(80 * 10);
const interest = new anchor.BN(5 * 10);

//request loan param
let nftWorth = new anchor.BN(10000)
let requestedAmount = new anchor.BN(7000)
// let duration =  new anchor.BN(5000)
let duration = new anchor.BN(slotsInAYear())


describe("🚀 cancel loan req", () => {
    // Configure the client to use the local cluster.
    // const provider = anchor.AnchorProvider.env()
    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);

    const program = anchor.workspace.P2PNftLending as Program<P2pNftLending>;

    before(async () => {



        owner1 = getKeypair("owner1");
        owner2 = getKeypair("owner2");
        owner3 = getKeypair("owner3");
        owner4 = getKeypair("owner4");
        owner5 = getKeypair("owner5");


        //only work in localnet
        let tx = new anchor.web3.Transaction().add(
            anchor.web3.SystemProgram.transfer({
                fromPubkey: provider.wallet.publicKey,
                toPubkey: owner1.publicKey,
                lamports: anchor.web3.LAMPORTS_PER_SOL * 1000,
            }),
            anchor.web3.SystemProgram.transfer({
                fromPubkey: provider.wallet.publicKey,
                toPubkey: owner2.publicKey,
                lamports: anchor.web3.LAMPORTS_PER_SOL * 1000,
            }),
            anchor.web3.SystemProgram.transfer({
                fromPubkey: provider.wallet.publicKey,
                toPubkey: owner3.publicKey,
                lamports: anchor.web3.LAMPORTS_PER_SOL * 10000,
            }),
            anchor.web3.SystemProgram.transfer({
                fromPubkey: provider.wallet.publicKey,
                toPubkey: owner4.publicKey,
                lamports: anchor.web3.LAMPORTS_PER_SOL * 1000,
            }),
            anchor.web3.SystemProgram.transfer({
                fromPubkey: provider.wallet.publicKey,
                toPubkey: owner5.publicKey,
                lamports: anchor.web3.LAMPORTS_PER_SOL * 1000,
            }),
        );

        await program.provider.sendAndConfirm(tx)



        requestedTokenMint = anchor.web3.PublicKey.default;

        newOwners = [owner1.publicKey, owner2.publicKey, owner5.publicKey];

        //Borrower
        if (requestedTokenMint == anchor.web3.PublicKey.default) {
            borrower = owner2;
            borrowerTokenAccount = borrower.publicKey;
        } else {
            borrowerTokenAccount = await createAssociatedTokenAccount(provider, requestedTokenMint, borrower);
        }
        nftMint = await createMint(provider, owner2,); //mint main nft

        writePublicKey(nftMint,"nftMint");
        
        [borrower, borrowerMainNftAccount] = await createAssociatedTokenAccountAndMintTo(provider, 1, nftMint, owner2); // borrower is owner2 
        borrowNftMint = await createMint(provider, borrower,);


        borrowNftAccount = await createAssociatedTokenAccount(provider, borrowNftMint, borrower);


        // requestedTokenMint = await createMint(provider, owner3,);
        if (requestedTokenMint == anchor.web3.PublicKey.default) {
            lender = owner3;
            lenderTokenAccount = lender.publicKey; //transfer lamports from lenders account
        } else {
            [lender, lenderTokenAccount] = await createAssociatedTokenAccountAndMintTo(provider, 10_000, requestedTokenMint, owner3);
        }
        lendNftMint = await createMint(provider, lender,);
        writePublicKey(lendNftMint,"lendNftMint");

        await sleep(100)
        lendNftAccount = await createAssociatedTokenAccount(provider, lendNftMint, lender);


        //state
        loanRequest = await getPdaParamsWithSeedAndPubkey(program as anchor.Program, LOAN_REQUEST_STR, borrowNftMint);
        grantLoan = await getPdaParamsWithSeedAndPubkey(program as anchor.Program, GRANT_LOAN_STR, lendNftMint);
        loan = await getLoanPDA(program as anchor.Program, LOAN_STR, loanRequest.key, grantLoan.key);
        loanFee = await (await getPdaParamsWithSeedAndPubkey(program as anchor.Program, LOAN_FEE_STR, loan)).key;

        multisigPda = await getPdaParams(program as anchor.Program, MULTISIG_SEED_STR);
        platformFees = await getPdaParams(program as anchor.Program, PLATFORM_FEES_SEED_STR);
        platformListing = await getPdaParams(program as anchor.Program, PLATFORM_LISTING_STR);

        //Escrow
        nftEscrowTokenAccount = await getPdaParamsWithSeedAndPubkey(program as anchor.Program, NFT_ESCROW_STR, loanRequest.key);
        // escrowTokenAccount = await getPdaParamsWithSeedAndPubkey(program as anchor.Program, LOAN_TOKEN_ESCROW, grantLoan.key);// lender is owner3 

        if (requestedTokenMint == anchor.web3.PublicKey.default) {
            // use pda here
            escrowTokenAccount = await findAssociatedTokenAddress(platformFees.key, requestedTokenMint, program.programId);
            feeEscrowTokenAccount = await findAssociatedTokenAddress(multisigPda.key, requestedTokenMint, program.programId);
        } else { //lendNftMint
            escrowTokenAccount = await findAssociatedTokenAddress(platformFees.key, requestedTokenMint);
            feeEscrowTokenAccount = await findAssociatedTokenAddress(multisigPda.key, requestedTokenMint);
        }

        await sleep(100);
    });


    it("rejects a borrow amount that when compounded exceeds loan to value", async () => {
        try {
            const requested = new anchor.BN(10000); //invalid amount
            await program.methods
                .requestForLoan(nftWorth, requested, duration)
                .accounts({
                    nftMint: nftMint,
                    nftTokenAccount: borrowerMainNftAccount,
                    borrowNftMint: borrowNftMint,
                    loanRequest: loanRequest.key,
                    borrowNftTokenAccount: borrowNftAccount,
                    requestedTokenMint: requestedTokenMint,
                    platformFees: platformFees.key,
                    nftEscrow: nftEscrowTokenAccount.key,
                    borrower: borrower.publicKey,
                })
                .signers([borrower])
                .rpc()
            assert.fail()
        } catch (e) {
            const error = e.error
            assert.strictEqual(error.errorCode.number, 6011);
            assert.strictEqual(error.errorMessage, 'Maximum borrow amount exceeded')
        }
    })
    it(" requests for loan (deposits collateral)", async () => {

        const initialMainNftBal = await provider.connection.getTokenAccountBalance(borrowerMainNftAccount,);
        const initialBorrowNftBal = await provider.connection.getTokenAccountBalance(borrowNftAccount,);


        assert.equal(parseInt(initialMainNftBal.value.amount), 1);
        assert.equal(parseInt(initialBorrowNftBal.value.amount), 0);

        await program.methods
            .requestForLoan(nftWorth, requestedAmount, duration)
            .accounts({
                nftMint: nftMint,
                nftTokenAccount: borrowerMainNftAccount,
                borrowNftMint: borrowNftMint,
                loanRequest: loanRequest.key,
                borrowNftTokenAccount: borrowNftAccount,
                requestedTokenMint: requestedTokenMint,
                platformFees: platformFees.key,
                nftEscrow: nftEscrowTokenAccount.key,
                borrower: borrower.publicKey,
            })
            .signers([borrower])
            .rpc()

        const loanRequestState = await program.account.loanRequest.fetch(loanRequest.key);
        assert.deepEqual(loanRequestState.borrowNftMint, borrowNftMint);
        // assert.equal(loanRequestState.feePercentage, fee.toNumber());
        // assert.equal(loanRequestState.interestRate, interest.toNumber());
        assert.equal(loanRequestState.slotDuration, duration.toNumber());
        assert.equal(loanRequestState.requestedAmount, requestedAmount.toNumber());
        assert.equal(loanRequestState.nftWorth, nftWorth.toNumber());


        await sleep(100); // Need to wait 1 sec to let method finish? not sure why


        const finalMainNftBal = await provider.connection.getTokenAccountBalance(borrowerMainNftAccount,);
        const finalBorrowNftBal = await provider.connection.getTokenAccountBalance(borrowNftAccount,);
        const finalEscrowNftBal = await provider.connection.getTokenAccountBalance(nftEscrowTokenAccount.key,);


        assert.equal(parseInt(finalMainNftBal.value.amount), 0);
        assert.equal(parseInt(finalBorrowNftBal.value.amount), 1);
        assert.equal(parseInt(finalEscrowNftBal.value.amount), 1);


    })





    it(" is able to generate back LoanRequest State using just Borrow nft token wallet", async () => {
        // borrowNftAccount
        const borrowNFTAcct = await tokenAccountInfo(provider, borrowNftAccount);
        assert.deepEqual(borrowNFTAcct.mint, borrowNftMint);

        const generatedLoanRequestAccount = await getPdaParamsWithSeedAndPubkey(program as anchor.Program, LOAN_REQUEST_STR, borrowNftMint);
        const loanRequestState = await program.account.loanRequest.fetch(generatedLoanRequestAccount.key)

        assert.equal(loanRequestState.slotDuration, duration.toNumber());
        assert.equal(loanRequestState.requestedAmount, requestedAmount.toNumber());
        assert.equal(loanRequestState.nftWorth, nftWorth.toNumber());

    })

    it("checks if user is able to borrow an amount against deposited NFT Collateral", async () => {
        const platformFeesState = await program.account.platformFees.fetch(platformFees.key);
        const nftWorth = 10000;
        const intendedBorrow = 4000;

        const compoundedValue = compoundInterest(
            intendedBorrow,
            platformFeesState.interestRate,
            duration.toNumber(),
        )

        const amountAllowed = maxAllowedAmount(
            nftWorth,
            platformFeesState.ltv
        );
        assert.ok(compoundedValue <= amountAllowed);

        // console.log("amount intended to borrow", intendedBorrow);
        // console.log("amount borrowed value will compound to after loan duration ", compoundedValue);
        // console.log("maximum amount allowed to be borrowed on this NFT", amountAllowed);

    })

    it("cancels loan request", async () => {
        await program.methods
            .cancelLoanRequest()
            .accounts(
                {
                    nftMint: nftMint,
                    nftTokenAccount: borrowerMainNftAccount,
                    borrowNftMint: borrowNftMint,
                    loanRequest: loanRequest.key,
                    borrowNftTokenAccount: borrowNftAccount,
                    requestedTokenMint: requestedTokenMint,
                    platformFees: platformFees.key,
                    nftEscrow: nftEscrowTokenAccount.key,
                    borrower: borrower.publicKey,
                }
            ).signers([borrower]).rpc();

        await sleep(100);

        const finalMainNftBal = await provider.connection.getTokenAccountBalance(borrowerMainNftAccount,);
        const finalBorrowNftBal = await provider.connection.getTokenAccountBalance(borrowNftAccount,);

        assert.equal(parseInt(finalMainNftBal.value.amount), 1);
        assert.equal(parseInt(finalBorrowNftBal.value.amount), 0);

        //nft escrow account closed
        try {
            await provider.connection.getTokenAccountBalance(nftEscrowTokenAccount.key,);
            assert.fail("nft escrow account closed");
        } catch (e) {
            // console.log(e.message);
            assert.equal(e.message, "failed to get token account balance: Invalid param: could not find account")
        }
        // loan request account closed
        try {
            await program.account.loanRequest.fetch(loanRequest.key)

        } catch (error) {
            // console.log(error.message);
            assert.equal(error.message, `Account does not exist ${loanRequest.key}`);

        }
        //make another loan request as previous request has been cancelled

    })

});