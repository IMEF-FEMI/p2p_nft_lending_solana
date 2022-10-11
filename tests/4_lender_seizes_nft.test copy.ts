import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { LAMPORTS_PER_SOL, } from "@solana/web3.js";
import assert from "assert";
import { P2pNftLending } from "../target/types/p2p_nft_lending";
import { maxAllowedAmount, compoundInterest, slotsInAYear, calculateFees, slotsInDuration } from "../test_utils/calculations";
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
let lenderMainNftAccount: anchor.web3.PublicKey;


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
let duration = new anchor.BN(slotsInDuration(2))
// console.log("duration, ", duration.toNumber(),);


describe("🚀 Lender Seizes NFT", () => {
    // Configure the client to use the local cluster.
    // const provider = anchor.AnchorProvider.env()
    const rpcHost = "http://127.0.0.1:8899";
    const connection = new anchor.web3.Connection(rpcHost, "confirmed");
    const provider = new anchor.AnchorProvider(
        connection,
        anchor.AnchorProvider.env().wallet,
        anchor.AnchorProvider.env().opts
    );
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
            borrower = owner2;
            borrowerTokenAccount = await createAssociatedTokenAccount(provider, requestedTokenMint, borrower);
        }

        nftMint = getPublicKey("nftMint"); //mint main nft
        borrowerMainNftAccount = await createAssociatedTokenAccount(provider, nftMint, borrower); // borrower is owner2 
        //set new borrow nft as the previous ownership has been transferred to program 
        // in previous test
        borrowNftMint = await createMint(provider, borrower,);
        await sleep(100);
        borrowNftAccount = await createAssociatedTokenAccount(provider, borrowNftMint, borrower);

        // requestedTokenMint = await createMint(provider, owner3,);
        if (requestedTokenMint == anchor.web3.PublicKey.default) {
            lender = owner3;
            lenderTokenAccount = lender.publicKey; //transfer lamports from lenders account
        } else {
            lender = owner3;

            lenderTokenAccount = await createAssociatedTokenAccount(provider, requestedTokenMint, lender);
            await mintTokens(provider, 10_000, requestedTokenMint, lender, lenderTokenAccount)
        }

        lendNftMint = await createMint(provider, lender,);
        writePublicKey(lendNftMint, "lendNftMint");

        lenderMainNftAccount = await createAssociatedTokenAccount(provider, nftMint, lender); // borrower is owner2 


        await sleep(100);
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
            assert.ok(feeEscrowBalance >= expectedFeesBalance * anchor.web3.LAMPORTS_PER_SOL)
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




    it("makes borrower default on loan payment", async () => {

        await sleep(2000)
        // pay half
        await program.methods.
            refreshLoan()
            .accounts({
                loan: loan,
            }).rpc();

        const loanState = await program.account.loan.fetch(loan)
        assert.equal(loanState.status, 3)
    })




    it("seizes nft when borrower defaults", async () => {
        await program.methods
            .seizeNft()
            .accounts({
                nftMint: nftMint,
                lenderNftAccount: lenderMainNftAccount,
                lender: lender.publicKey,
                lendNftMint,
                lendNftAccount,
                loan,
                nftEscrow: nftEscrowTokenAccount.key,
                platformFees: platformFees.key,
                grantLoanReq: grantLoan.key,
            })
            .signers([lender])
            .rpc().catch(err => console.error(err))

            const loanState = await program.account.loan.fetch(loan)
            assert.equal(loanState.status, 4)
    })
});
