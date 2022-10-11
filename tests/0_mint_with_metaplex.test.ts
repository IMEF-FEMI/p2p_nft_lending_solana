import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { P2pNftLending } from "../target/types/p2p_nft_lending";
import { main_nft_uri } from "../test_utils/CONSTANTS";
import { mintLoanNFTUsingMetaplex, uploadImageAndMetadataToArweave } from "../test_utils/mintNFTUtils";

describe("mint with metaplex", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.P2PNftLending as Program<P2pNftLending>;

  it("Is Mints NFT needed for collateral", async () => {

    // await uploadImageAndMetadataToArweave()
    // await mintLoanNFTUsingMetaplex(program as Program, "Main Nft", "owner1", main_nft_uri)

  });
});
