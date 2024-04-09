use std::ops::Deref;

use reth_primitives::{revm_primitives::bitvec::view::BitViewSized, Address, U256};
use serde::{Deserialize, Serialize};

use crate::keccak::{keccak256, keccak256_combine, Digest as KeccakDigest};

/// Encapsulates the information to uniquely identify a token on the origin network.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TokenInfo {
    /// Network which the token originates from
    pub origin_network: NetworkId,
    /// The address of the token on the origin network
    pub origin_token_address: Address,
}

impl TokenInfo {
    /// Computes the Keccak digest of [`TokenInfo`].
    pub fn hash(&self) -> KeccakDigest {
        keccak256_combine([
            &self.origin_network.to_be_bytes(),
            self.origin_token_address.as_slice(),
        ])
    }
}

/// Represents a token withdrawal from the network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Withdrawal {
    pub leaf_type: u8,

    /// Unique ID for the token being transferred.
    pub token_info: TokenInfo,

    /// Network which the token is transfered to
    pub dest_network: NetworkId,
    /// Address which will own the received token
    pub dest_address: Address,

    /// Token amount sent
    pub amount: U256,

    pub metadata: Vec<u8>,
}

impl Withdrawal {
    /// Creates a new [`Withdrawal`].
    pub fn new(
        leaf_type: u8,
        origin_network: NetworkId,
        origin_token_address: Address,
        dest_network: NetworkId,
        dest_address: Address,
        amount: U256,
        metadata: Vec<u8>,
    ) -> Self {
        Self {
            leaf_type,
            token_info: TokenInfo {
                origin_network,
                origin_token_address,
            },
            dest_network,
            dest_address,
            amount,
            metadata,
        }
    }

    /// Hashes the [`Withdrawal`] to be inserted in a [`crate::local_exit_tree::LocalExitTree`].
    pub fn hash(&self) -> KeccakDigest {
        keccak256_combine([
            self.leaf_type.as_raw_slice(),
            &u32::to_be_bytes(self.token_info.origin_network.into()),
            self.token_info.origin_token_address.as_slice(),
            &u32::to_be_bytes(self.dest_network.into()),
            self.dest_address.as_slice(),
            &self.amount_as_bytes(),
            &keccak256(&self.metadata),
        ])
    }

    /// Prepares the `amount` field for hashing
    fn amount_as_bytes(&self) -> [u8; 32] {
        let amount_bytes = self.amount.to_be_bytes::<32>();
        let padding_length = 32 - amount_bytes.len();

        let mut output = Vec::with_capacity(32);
        output.resize(padding_length, 0_u8);
        output.extend_from_slice(&amount_bytes);

        output.try_into().unwrap()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NetworkId(u32);

impl NetworkId {
    pub fn new(value: u32) -> Self {
        Self(value)
    }
}

impl From<u32> for NetworkId {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<NetworkId> for u32 {
    fn from(value: NetworkId) -> Self {
        value.0
    }
}

impl Deref for NetworkId {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::local_exit_tree::{hasher::Keccak256Hasher, LocalExitTree};

    #[test]
    fn test_deposit_hash() {
        let mut deposit = Withdrawal::new(
            0,
            0.into(),
            Address::default(),
            1.into(),
            Address::default(),
            U256::default(),
            vec![],
        );

        let amount_bytes = hex::decode("8ac7230489e80000").unwrap_or_default();
        deposit.amount = U256::try_from_be_slice(amount_bytes.as_slice()).unwrap();

        let dest_addr = hex::decode("c949254d682d8c9ad5682521675b8f43b102aec4").unwrap_or_default();
        deposit.dest_address.copy_from_slice(&dest_addr);

        let leaf_hash = deposit.hash();
        assert_eq!(
            "22ed288677b4c2afd83a6d7d55f7df7f4eaaf60f7310210c030fd27adacbc5e0",
            hex::encode(leaf_hash)
        );

        let mut dm = LocalExitTree::<Keccak256Hasher>::new();
        dm.add_leaf(leaf_hash);
        let dm_root = dm.get_root();
        assert_eq!(
            "5ba002329b53c11a2f1dfe90b11e031771842056cf2125b43da8103c199dcd7f",
            hex::encode(dm_root)
        );
    }
}
