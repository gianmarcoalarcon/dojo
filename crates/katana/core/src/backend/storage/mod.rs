use std::collections::{HashMap, VecDeque};

use blockifier::block_context::BlockContext;
use starknet::core::types::{BlockId, BlockTag, FieldElement};

use self::{block::Block, transaction::KnownTransaction};
use super::state::MemDb;
use crate::backend::storage::block::PartialHeader;

pub mod block;
pub mod transaction;

/// Represents the complete state of a single block
pub struct InMemoryBlockStates {
    /// The states at a certain block
    states: HashMap<FieldElement, MemDb>,
    /// How many states to store at most
    in_memory_limit: usize,
    /// minimum amount of states we keep in memory
    min_in_memory_limit: usize,
    /// all states present, used to enforce `in_memory_limit`
    present: VecDeque<FieldElement>,
}

impl InMemoryBlockStates {
    pub fn new(limit: usize) -> Self {
        Self {
            states: Default::default(),
            in_memory_limit: limit,
            min_in_memory_limit: limit.min(10),
            present: Default::default(),
        }
    }

    /// Inserts a new (hash -> state) pair
    ///
    /// When the configured limit for the number of states that can be stored in memory is reached,
    /// the oldest state is removed.
    ///
    /// Since we keep a snapshot of the entire state as history, the size of the state will increase
    /// with the transactions processed. To counter this, we gradually decrease the cache limit with
    /// the number of states/blocks until we reached the `min_limit`.
    pub fn insert(&mut self, hash: FieldElement, state: MemDb) {
        if self.present.len() >= self.in_memory_limit {
            // once we hit the max limit we gradually decrease it
            self.in_memory_limit =
                self.in_memory_limit.saturating_sub(1).max(self.min_in_memory_limit);
        }

        self.enforce_limits();
        self.states.insert(hash, state);
        self.present.push_back(hash);
    }

    /// Enforces configured limits
    fn enforce_limits(&mut self) {
        // enforce memory limits
        while self.present.len() >= self.in_memory_limit {
            // evict the oldest block in memory
            if let Some(hash) = self.present.pop_front() {
                self.states.remove(&hash);
            }
        }
    }
}

// TODO: can we wrap all the fields in a `RwLock` to prevent read blocking?
#[derive(Debug, Default)]
pub struct BlockchainStorage {
    /// Mapping from block hash -> block
    pub blocks: HashMap<FieldElement, Block>,
    /// Mapping from block number -> block hash
    pub hashes: HashMap<u64, FieldElement>,
    /// The latest block hash
    pub latest_hash: FieldElement,
    /// The latest block number
    pub latest_number: u64,
    /// Mapping of all known transactions from its transaction hash
    pub transactions: HashMap<FieldElement, KnownTransaction>,
}

impl BlockchainStorage {
    /// Creates a new blockchain from a genesis block
    pub fn new(block_context: &BlockContext) -> Self {
        let partial_header = PartialHeader {
            parent_hash: FieldElement::ZERO,
            gas_price: block_context.gas_price,
            number: block_context.block_number.0,
            timestamp: block_context.block_timestamp.0,
            sequencer_address: (*block_context.sequencer_address.0.key()).into(),
        };

        // Create a dummy genesis block
        let genesis_block = Block::new(partial_header, vec![], vec![]);
        let genesis_hash = genesis_block.header.hash();
        let genesis_number = 0u64;

        Self {
            blocks: HashMap::from([(genesis_hash, genesis_block)]),
            hashes: HashMap::from([(genesis_number, genesis_hash)]),
            latest_hash: genesis_hash,
            latest_number: genesis_number,
            transactions: HashMap::default(),
        }
    }

    pub fn total_blocks(&self) -> usize {
        self.blocks.len()
    }

    /// Returns the block hash based on the block id
    pub fn hash(&self, block: BlockId) -> Option<FieldElement> {
        match block {
            BlockId::Tag(BlockTag::Pending) => None,
            BlockId::Tag(BlockTag::Latest) => Some(self.latest_hash),
            BlockId::Hash(hash) => Some(hash),
            BlockId::Number(num) => self.hashes.get(&num).copied(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn remove_old_state_when_limit_is_reached() {
        let mut in_memory_state = InMemoryBlockStates::new(2);

        in_memory_state.insert(FieldElement::from_str("0x1").unwrap(), MemDb::default());
        in_memory_state.insert(FieldElement::from_str("0x2").unwrap(), MemDb::default());
        assert!(in_memory_state.states.get(&FieldElement::from_str("0x1").unwrap()).is_some());
        assert!(in_memory_state.states.get(&FieldElement::from_str("0x2").unwrap()).is_some());
        assert_eq!(in_memory_state.present.len(), 2);

        in_memory_state.insert(FieldElement::from_str("0x3").unwrap(), MemDb::default());

        assert_eq!(in_memory_state.present.len(), 2);
        assert!(in_memory_state.states.get(&FieldElement::from_str("0x1").unwrap()).is_none());
        assert!(in_memory_state.states.get(&FieldElement::from_str("0x2").unwrap()).is_some());
        assert!(in_memory_state.states.get(&FieldElement::from_str("0x3").unwrap()).is_some());
    }
}