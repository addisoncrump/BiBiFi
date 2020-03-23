use arrayref::array_ref;
use blake2::{Blake2s, Digest};

pub fn hash(input: String) -> [u8; 32] {
    let mut hasher = Blake2s::new();
    hasher.input(input);
    let res = hasher.result();
    *array_ref!(res.as_slice(), 0, 32)
}
