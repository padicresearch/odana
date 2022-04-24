use crypto::SHA256;
use primitive_types::H256;

const LEAF_PREFIX: [u8; 1] = [0];
const NODE_PREFIX: [u8; 1] = [1];
pub const HASH_LEN: usize = 32;

pub(crate) trait TreeHasher {
    #[inline]
    fn digest(&self, data: &[u8]) -> H256 {
        SHA256::digest(data)
    }

    #[inline]
    fn path(&self, key: &[u8]) -> H256 {
        self.digest(key)
    }

    #[inline]
    fn digest_leaf(&self, path: &[u8], leaf_data: &[u8]) -> (H256, Vec<u8>) {
        let mut value = Vec::with_capacity(LEAF_PREFIX.len() + path.len() + leaf_data.len());
        value.extend_from_slice(&LEAF_PREFIX);
        value.extend_from_slice(path);
        value.extend_from_slice(leaf_data);

        let sum = SHA256::digest(&value);
        (sum, value)
    }

    #[inline]
    fn digest_node(&self, left_data: &[u8], right_data: &[u8]) -> (H256, Vec<u8>) {
        let mut value = Vec::with_capacity(NODE_PREFIX.len() + left_data.as_ref().len() + right_data.as_ref().len());
        value.extend_from_slice(&NODE_PREFIX);
        value.extend_from_slice(left_data);
        value.extend_from_slice(right_data);

        let sum = SHA256::digest(&value);
        (sum, value)
    }

    #[inline]
    fn parse_leaf<'a>(&'a self, data: &'a [u8]) -> (&'a [u8], &'a [u8]) {
        (&data[LEAF_PREFIX.len()..HASH_LEN + LEAF_PREFIX.len()], &data[LEAF_PREFIX.len() + HASH_LEN..])
    }

    #[inline]
    fn parse_node<'a>(&'a self, data: &'a [u8]) -> (&'a [u8], &'a [u8]) {
        (&data[NODE_PREFIX.len()..HASH_LEN + NODE_PREFIX.len()], &data[NODE_PREFIX.len() + HASH_LEN..])
    }

    #[inline]
    fn is_leaf(&self, data: &[u8]) -> bool {
        data[..NODE_PREFIX.len()] == LEAF_PREFIX
    }

    #[inline]
    fn placeholder(&self) -> H256 {
        H256::zero()
    }

    #[inline]
    fn path_size(&self) -> usize {
        return HASH_LEN;
    }
}
