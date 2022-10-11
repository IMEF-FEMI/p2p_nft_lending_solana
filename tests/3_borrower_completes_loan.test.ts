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
    getAssociatedTokenAddressOnly,
    getKeypair,
    getLoanPDA,
    getMultisigTransactionPdaParams,
    getPdaParams,
    getPdaParamsWithSeedAndPubkey,
    getPublicKey,
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


describe("🚀 borrower completes loan", () => {
    // Configure the client to use the local cluster.
    const provider = anchor.AnchorProvider.env()
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


        //    await program.provider.connection.requestAirdrop(
        //         owner1.publicKey,
        //         anchor.web3.LAMPORTS_PER_SOL * 1000,
        //     );

        //     await program.provider.connection.requestAirdrop(
        //         owner2.publicKey,
        //         anchor.web3.LAMPORTS_PER_SOL * 1000,
        //     );
        //     await program.provider.connection.requestAirdrop(
        //         owner3.publicKey,
        //         anchor.web3.LAMPORTS_PER_SOL * 1000,
        //     );
        //     await program.provider.connection.requestAirdrop(
        //         owner4.publicKey,
        //         anchor.web3.LAMPORTS_PER_SOL * 1000,
        //     );
        //     await program.provider.connection.requestAirdrop(
        //         owner5.publicKey,
        //         anchor.web3.LAMPORTS_PER_SOL * 1000,
        //     );
        requestedTokenMint = anchor.web3.PublicKey.default;

        newOwners = [owner1.publicKey, owner2.publicKey, owner5.publicKey];

        //Borrower
        if (requestedTokenMint == anchor.web3.PublicKey.default) {
            borrower = owner2;
            borrowerTokenAccount = borrower.publicKey;
        } else {
            borrowerTokenAccount = await getAssociatedTokenAddressOnly(requestedTokenMint, borrower.publicKey);
        }
        nftMint = getPublicKey("nftMint"); //mint main nft
        borrowerMainNftAccount = await getAssociatedTokenAddressOnly(nftMint, borrower.publicKey); // borrower is owner2 
        //set new borrow nft as the previous ownership has been transferred to program 
        // in previous test
        borrowNftMint = await createMint(provider, borrower,);
        writePublicKey(borrowNftMint, "borrowNftMint");
        borrowNftAccount = await createAssociatedTokenAccount(provider, borrowNftMint, borrower);

        // requestedTokenMint = await createMint(provider, owner3,);
        if (requestedTokenMint == anchor.web3.PublicKey.default) {
            lender = owner3;
            lenderTokenAccount = lender.publicKey; //transfer lamports from lenders account
        } else {
            lender = owner3;

            lenderTokenAccount = await getAssociatedTokenAddressOnly(requestedTokenMint, owner3.publicKey);
            await mintTokens(provider, 10_000, requestedTokenMint, owner3, lenderTokenAccount)
        }

        lendNftMint = getPublicKey("lendNftMint");
        // borrowNftMint = await createMint(provider, borrower,);


        lendNftAccount = await getAssociatedTokenAddressOnly(lendNftMint, lender.publicKey);


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


    it("grants loan request", async () => {

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
            .rpc().catch(err => console.log(err))

        await program.methods
            .grantLoan()
            .accounts({
                lendNftMint: lendNftMint,
                lendNftAccount: lendNftAccount,
                requestedTokenMint: requestedTokenMint,
                requestedTokenAccount: lenderTokenAccount,
                loanRequest: loanRequest.key,
                grantLoanReq: grantLoan.key,
                platformFees: platformFees.key,
                loan: loan,
                loanFee: loanFee,
                loanFeeEscrow: feeEscrowTokenAccount,
                multisig: multisigPda.key,
                loanTokenEscrow: escrowTokenAccount,
                lender: lender.publicKey,
            }).
            signers([lender])
            .rpc().catch(err => {
                console.log(err);
            })


        await sleep(100);
        const loanRequestState = await program.account.loanRequest.fetch(loanRequest.key);
        const platformFeesState = await program.account.platformFees.fetch(platformFees.key);
        const expectedFeesBalance = calculateFees(loanRequestState.requestedAmount.toNumber(), platformFeesState.feePercentage);
        const expectedEscrowBalance = loanRequestState.requestedAmount.toNumber() - expectedFeesBalance;

        if (requestedTokenMint == anchor.web3.PublicKey.default) {
            // SOL
            const escrowBalance = await provider.connection.getBalance(escrowTokenAccount);
            const feeEscrowBalance = await provider.connection.getBalance(feeEscrowTokenAccount);
            assert.equal(escrowBalance, expectedEscrowBalance * anchor.web3.LAMPORTS_PER_SOL)
            assert.equal(feeEscrowBalance, expectedFeesBalance * anchor.web3.LAMPORTS_PER_SOL)
        } else {
            const escrowBalance = await provider.connection.getTokenAccountBalance(escrowTokenAccount,);
            const feeEscrowBalance = await provider.connection.getTokenAccountBalance(escrowTokenAccount,);
            assert.equal(escrowBalance, expectedEscrowBalance)
            assert.equal(feeEscrowBalance, expectedFeesBalance)
        }


        const lendNftBal = await provider.connection.getTokenAccountBalance(lendNftAccount,);
        assert.equal(parseInt(lendNftBal.value.amount), 1);

        const loanState = await program.account.loan.fetch(loan);
        assert.deepEqual(loanState.nftMint, nftMint)
        assert.deepEqual(loanState.borrowNftMint, borrowNftMint)
        assert.deepEqual(loanState.lendNftMint, lendNftMint)
        assert.deepEqual(loanRequestState.loan, loan)
        assert.equal(loanState.ltv, ltv.toNumber())
        assert.equal(loanState.feePercentage, fee.toNumber())
        assert.equal(loanState.interestRate, interest.toNumber())


    })

    it("Borrower Withdraws Granted loan ", async () => {
        const initialBorrowersBal = await provider.connection.getBalance(borrower.publicKey);

        await program.methods
            .borrowerWithdrawTokens()
            .accounts({
                requestedTokenAccount: borrowerTokenAccount,
                loanRequest: loanRequest.key,
                platformFees: platformFees.key,
                loan: loan,
                borrower: borrower.publicKey,
                loanTokenEscrow: escrowTokenAccount
            }).
            signers([borrower])
            .rpc()

        await sleep(100);
        const loanRequestState = await program.account.loanRequest.fetch(loanRequest.key);

        const platformFeesState = await program.account.platformFees.fetch(platformFees.key);
        const fee = calculateFees(loanRequestState.requestedAmount.toNumber(), platformFeesState.feePercentage);

        if (requestedTokenMint == anchor.web3.PublicKey.default) {
            // SOL
            const newEscrowBalance = await provider.connection.getBalance(escrowTokenAccount);
            assert.equal(newEscrowBalance, 0)

            const newBorrowersBal = await provider.connection.getBalance(borrowerTokenAccount);
            assert.equal(newBorrowersBal, initialBorrowersBal + (loanRequestState.requestedAmount.toNumber() - fee) * anchor.web3.LAMPORTS_PER_SOL);
        } else {
            const newEscrowBalance = await provider.connection.getTokenAccountBalance(escrowTokenAccount,);
            assert.equal(newEscrowBalance, 0)
            const newBorrowersBal = await provider.connection.getTokenAccountBalance(borrowerTokenAccount,);
            assert.equal(newBorrowersBal, (loanRequestState.requestedAmount.toNumber() - fee))
        }
        const loanState = await program.account.loan.fetch(loan)
        assert.equal(loanState.status, 1)
    })

    it("Admin acct withdraws a portion of fees taken from a loan ", async () => {

        let adminTokenAccount: anchor.web3.PublicKey;
        let oldAdminBal: number;
        //lets use owner1
        if (requestedTokenMint == anchor.web3.PublicKey.default) {
            adminTokenAccount = owner1.publicKey;
            oldAdminBal = await provider.connection.getBalance(adminTokenAccount);
        } else {
            adminTokenAccount = await createAssociatedTokenAccount(provider, requestedTokenMint, owner1);
            oldAdminBal = Number(await (await provider.connection.getTokenAccountBalance(adminTokenAccount,)).value.amount);
        }

        await program.methods
            .withdrawFee()
            .accounts({
                platformFees: platformFees.key,
                loan: loan,
                loanFee: loanFee,
                loanFeeEscrow: feeEscrowTokenAccount,
                multisig: multisigPda.key,
                adminTokenAccount,
                admin: owner1.publicKey,
            })
            .signers([owner1])
            .rpc();

        await sleep(100);

        if (requestedTokenMint == anchor.web3.PublicKey.default) {
            const newAdminBal = await provider.connection.getBalance(adminTokenAccount);
            assert.ok(newAdminBal > oldAdminBal);
        } else {
            const newAdminBal = await provider.connection.getTokenAccountBalance(adminTokenAccount,);
            assert.equal(newAdminBal, oldAdminBal);
        }
    })

    it("admin cannot withdraw twice", async () => {
        let adminTokenAccount: anchor.web3.PublicKey;
        //lets use owner1
        if (requestedTokenMint == anchor.web3.PublicKey.default) {
            adminTokenAccount = owner1.publicKey;
        } else {
            adminTokenAccount = await createAssociatedTokenAccount(provider, requestedTokenMint, owner1);
        }

        try {
            await program.methods
                .withdrawFee()
                .accounts({
                    platformFees: platformFees.key,
                    loan: loan,
                    loanFee: loanFee,
                    loanFeeEscrow: feeEscrowTokenAccount,
                    multisig: multisigPda.key,
                    adminTokenAccount,
                    admin: owner1.publicKey,
                })
                .signers([owner1])
                .rpc();
        } catch (err) {
            const error = err.error
            assert.strictEqual(error.errorCode.number, 6018);
            assert.strictEqual(error.errorMessage, 'Admin(owner) has already withdrawn allocated fees')

        }
    })
    it("partially repays loan", async () => {
        const loanRequestState = await program.account.loanRequest.fetch(loanRequest.key);

        let initialLoanState = await program.account.loan.fetch(loan)

        if (requestedTokenMint != anchor.web3.PublicKey.default) {

            // mint spl tokens with loanRequestState.requestedTokenMint
            // to borrowerTokenAccount
            await mintTokens(
                provider,
                10_000,
                loanRequestState.requestedTokenMint,
                borrower,
                borrowerTokenAccount
            );
        }
        // pay half
        await program.methods
            .repayLoan(requestedAmount.div(new anchor.BN(2)))
            .accounts({
                requestedTokenAccount: borrowerTokenAccount,
                loanRequest: loanRequest.key,
                platformFees: platformFees.key,
                loanTokenEscrow: escrowTokenAccount,
                loan: loan,
                borrowNftMint: borrowNftMint,
                borrowNftTokenAccount: borrowNftAccount,
                nftMint: nftMint,
                nftTokenAccount: borrowerMainNftAccount,
                nftEscrow: nftEscrowTokenAccount.key,
                borrower: borrower.publicKey
            }).signers([borrower]).rpc();

        let finalLoanState = await program.account.loan.fetch(loan)
        assert.deepEqual(finalLoanState.outstandingDebt.toNumber(), initialLoanState.outstandingDebt.div(new anchor.BN(2)).toNumber());


        assert.ok(finalLoanState.outstandingDebt.toNumber() < initialLoanState.outstandingDebt.toNumber());
        await sleep(100);
        //check escrow escrowTokenAccount balance
        if (requestedTokenMint == anchor.web3.PublicKey.default) {
            const newEscrowBalance = await provider.connection.getBalance(escrowTokenAccount);
            assert.ok(newEscrowBalance / LAMPORTS_PER_SOL == initialLoanState.outstandingDebt.div(new anchor.BN(2)).toNumber());
        } else {
            const newEscrowBalance = await provider.connection.getTokenAccountBalance(escrowTokenAccount,);
            assert.equal(newEscrowBalance, initialLoanState.outstandingDebt.div(new anchor.BN(2)).toNumber());
        }
    })

    it("fully repay loan", async () => {
        await program.methods
            .repayLoan(requestedAmount)
            .accounts({
                requestedTokenAccount: borrowerTokenAccount,
                loanRequest: loanRequest.key,
                platformFees: platformFees.key,
                loanTokenEscrow: escrowTokenAccount,
                loan: loan,
                borrowNftMint: borrowNftMint,
                borrowNftTokenAccount: borrowNftAccount,
                nftMint: nftMint,
                nftTokenAccount: borrowerMainNftAccount,
                nftEscrow: nftEscrowTokenAccount.key,
                borrower: borrower.publicKey
            }).signers([borrower]).rpc();
        let finalLoanState = await program.account.loan.fetch(loan)

        await sleep(100);
        assert.equal(finalLoanState.outstandingDebt.toNumber(), 0);

        const finalMainNftBal = await provider.connection.getTokenAccountBalance(borrowerMainNftAccount,);
        const finalBorrowNftBal = await provider.connection.getTokenAccountBalance(borrowNftAccount,);
        const finalEscrowNftBal = await provider.connection.getTokenAccountBalance(nftEscrowTokenAccount.key,);


        assert.equal(parseInt(finalMainNftBal.value.amount), 1);
        assert.equal(parseInt(finalBorrowNftBal.value.amount), 0);
        assert.equal(parseInt(finalEscrowNftBal.value.amount), 0);

    })

    it("lender takes back borrowed tokens with interest", async () => {
        let oldLenderBal: number;

        if (requestedTokenMint == anchor.web3.PublicKey.default) {
            oldLenderBal = await provider.connection.getBalance(lenderTokenAccount);
        } else {
            oldLenderBal = Number(await (await provider.connection.getTokenAccountBalance(lenderTokenAccount,)).value.amount);
        }
        await program.methods
            .lenderWithdrawTokens()
            .accounts({
                requestedTokenAccount: lenderTokenAccount,
                // loanRequest: loanRequest.key,
                platformFees: platformFees.key,
                loan: loan,
                lender: lender.publicKey,
                loanTokenEscrow: escrowTokenAccount,
                lendNftMint: lendNftMint,
                lendNftAccount: lendNftAccount,
                requestedTokenMint: requestedTokenMint,
                grantLoanReq: grantLoan.key,
                multisig: multisigPda.key,
            }).
            signers([lender])
            .rpc()
            
            await sleep(100);

        const finalLendNftBal = await provider.connection.getTokenAccountBalance(lendNftAccount,);
        assert.equal(parseInt(finalLendNftBal.value.amount), 0);



        if (requestedTokenMint == anchor.web3.PublicKey.default) {
            const newBal = await provider.connection.getBalance(lenderTokenAccount);
            // const newEscrowBalance = await provider.connection.getBalance(escrowTokenAccount);
            // console.log("----------------------------------------------------------------" + newEscrowBalance);
            assert.ok(newBal > oldLenderBal );
        } else {
            const newBal = await provider.connection.getTokenAccountBalance(lenderTokenAccount,);
            assert.equal(newBal, oldLenderBal);
        }
    })

    

});
