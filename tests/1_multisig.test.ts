import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import assert from "assert";
import { P2pNftLending } from "../target/types/p2p_nft_lending";
import {
    MULTISIG_SEED_STR,
    PLATFORM_FEES_SEED_STR,
    PLATFORM_LISTING_STR
} from "../test_utils/CONSTANTS";
import { PDAParameters } from "../test_utils/types";
import {
    getKeypair,
    getMultisigTransactionPdaParams,
    getPdaParams,
} from "../test_utils/walletUtils";


let owner1: anchor.web3.Keypair;
let owner2: anchor.web3.Keypair;
let owner3: anchor.web3.Keypair;
let owner4: anchor.web3.Keypair;
let owner5: anchor.web3.Keypair;




// PDAs
let platformFees: PDAParameters;
let platformListing: PDAParameters;
let multisigPda: PDAParameters;


const threshold = new anchor.BN(2);
//fees
const fee = new anchor.BN(3 * 10);
const ltv = new anchor.BN(80 * 10);
const interest = new anchor.BN(5 * 10);

//request loan param
// let duration =  new anchor.BN(5000)


describe("🚀 Multisig", () => {
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




        multisigPda = await getPdaParams(program as anchor.Program, MULTISIG_SEED_STR);
        platformFees = await getPdaParams(program as anchor.Program, PLATFORM_FEES_SEED_STR);
        platformListing = await getPdaParams(program as anchor.Program, PLATFORM_LISTING_STR);

        //Escrow
    


    });

    it("assert that there are no duplicate accounts as owners when creating a multisig", async () => {
        const owners = [owner1.publicKey, owner2.publicKey, owner2.publicKey];



        try {
            await program.methods.initializeMultisig(owners, threshold,)
                .accounts({
                    multisig: multisigPda.key,
                    payer: provider.wallet.publicKey,
                    platformFees: platformFees.key,
                    platformListing: platformListing.key
                }).rpc();
            assert.fail();
        } catch (err) {
            const error = err.error
            assert.strictEqual(error.errorCode.number, 6002);
            assert.strictEqual(error.errorMessage, 'Owners must be unique')
        }
    })

    it("creates platform multisig", async () => {

        const owners = [owner1.publicKey, owner2.publicKey, owner3.publicKey];
        // let listener = null;

        // let [event, slot] = await new Promise((resolve, _reject) => {
        //     listener = program.addEventListener("MultisigCreated", (event, slot) => {
        //         resolve([event, slot]);
        //     });

        await program.methods.initializeMultisig(owners, threshold,)
            .accounts({
                multisig: multisigPda.key,
                payer: provider.wallet.publicKey,
                platformFees: platformFees.key,
                platformListing: platformListing.key
            }).rpc();
        // })

        // await program.removeEventListener(listener);



        const createdMultisig = await program.account.multisig.fetch(multisigPda.key)
        // const createdPlatformFeesAcct = await program.account.platformFees.fetch(platformFees.key);

        assert.strictEqual(createdMultisig.seqno, 0);
        assert.ok(createdMultisig.threshold.eq(new anchor.BN(2)));
        assert.deepEqual(createdMultisig.owners, owners)

        // assert.strictEqual(createdPlatformFeesAcct.feePercentage, fee.toNumber())
        // assert.strictEqual(createdPlatformFeesAcct.ltv, ltv.toNumber())
        // assert.strictEqual(createdPlatformFeesAcct.interestRate, interest.toNumber())
    })


    it("💰 set platform fees", async () => {


        const accounts = [
            {
                pubkey: platformFees.key,
                isWritable: true,
                isSigner: false
            },
            {
                pubkey: multisigPda.key,
                isWritable: false,
                isSigner: true
            }
        ];

        const multisigData = await program.account.multisig.fetch(multisigPda.key)

        const data = program.coder.instruction.encode("set_platform_fees", {
            feePercentage: fee.toNumber(),
            interestRate: interest.toNumber(),
            ltv: ltv.toNumber(),
        })


        const multisigTxPda = await getMultisigTransactionPdaParams(program, multisigData.seqno);

        await program.methods.createTransaction(program.programId, accounts, data)
            .accounts({
                multisig: multisigPda.key,

                transaction: multisigTxPda.key,
                proposer: owner1.publicKey,
            })
            .signers([owner1])
            .rpc()

        await program.methods
            .approve()
            .accounts({
                multisig: multisigPda.key,
                transaction: multisigTxPda.key,
                owner: owner2.publicKey
            })
            .signers([owner2])
            .rpc()

        await program.methods
            .executeTransaction()
            .accounts({
                multisig: multisigPda.key,
                multisigSigner: multisigPda.key,
                transaction: multisigTxPda.key,
                proposer: owner1.publicKey,
            })
            .remainingAccounts(
                accounts.map(
                    account => account.pubkey.equals(multisigPda.key) ?
                        { ...account, isSigner: false } : account
                )
                    .concat({
                        pubkey: program.programId,
                        isWritable: false,
                        isSigner: false,
                    }))
            .rpc()

        let platformFeesAccount = await program.account.platformFees.fetch(platformFees.key);
        assert.ok(platformFeesAccount.feePercentage === fee.toNumber());
        assert.ok(platformFeesAccount.ltv === ltv.toNumber());
        assert.ok(platformFeesAccount.interestRate === interest.toNumber());

    })



    it("Creates a multisig transaction", async () => {
        const newOwners = [owner1.publicKey, owner2.publicKey, owner5.publicKey];

        const pid = program.programId;

        const accounts = [{
            pubkey: multisigPda.key,
            isWritable: true,
            isSigner: false
        },
        {
            pubkey: multisigPda.key,
            isWritable: false,
            isSigner: true
        }];

        const multisigData = await program.account.multisig.fetch(multisigPda.key)

        const data = program.coder.instruction.encode("set_owners", {
            owners: newOwners,
        })

        // const data = program.coder.instruction.encode("set_owners_and_change_threshold", {
        //     owners: newOwnersWithOwner5,
        //     threshold: 2
        // })

        const multisigTxPda = await getMultisigTransactionPdaParams(program, multisigData.seqno);

        await program.methods.createTransaction(program.programId, accounts, data)
            .accounts({
                multisig: multisigPda.key,

                transaction: multisigTxPda.key,
                proposer: owner1.publicKey,
            })
            .signers([owner1])
            .rpc()

        const txAccount = await program.account.transaction.fetch(
            multisigTxPda.key,
        )


        assert.ok(txAccount.programId.equals(pid));
        assert.deepEqual(txAccount.accounts, accounts);
        assert.deepEqual(txAccount.data, data);
        assert.ok(txAccount.multisig.equals(multisigPda.key));
        assert.deepEqual(txAccount.didExecute, false);
        assert.ok(txAccount.seqno === 0);
    })
    it("approve and execute the multisig transaction", async () => {
        const multisigData = await program.account.multisig.fetch(multisigPda.key)

        const multisigTxPda = await getMultisigTransactionPdaParams(program, multisigData.seqno);

        await program.methods
            .approve()
            .accounts({
                multisig: multisigPda.key,
                transaction: multisigTxPda.key,
                owner: owner2.publicKey
            })
            .signers([owner2])
            .rpc()

        await program.methods
            .executeTransaction()
            .accounts({
                multisig: multisigPda.key,
                multisigSigner: multisigPda.key,
                transaction: multisigTxPda.key,
                proposer: owner1.publicKey,
            })
            .remainingAccounts([
                {
                    pubkey: multisigPda.key,
                    isWritable: true,
                    isSigner: false,
                },
                {
                    pubkey: multisigPda.key,
                    isWritable: false,
                    isSigner: false,
                },
                {
                    pubkey: program.programId,
                    isWritable: false,
                    isSigner: false,
                }
            ])
            .rpc()

        let multisigAccount = await program.account.multisig.fetch(multisigPda.key);
        assert.ok(multisigAccount.seqno === 1);


    })
});