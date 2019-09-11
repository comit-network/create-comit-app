use quickcheck::{self, Arbitrary};
use web3::types::{H160, U256};

#[derive(Clone, Debug)]
pub struct Quickcheck<I>(pub I);

impl From<Quickcheck<H160>> for H160 {
    fn from(source: Quickcheck<H160>) -> Self {
        source.0
    }
}

impl Arbitrary for Quickcheck<H160> {
    fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
        let mut inner = [0u8; 20];
        g.fill_bytes(&mut inner);

        Quickcheck(H160::from(&inner))
    }
}

impl From<Quickcheck<U256>> for U256 {
    fn from(source: Quickcheck<U256>) -> Self {
        source.0
    }
}

impl Arbitrary for Quickcheck<U256> {
    fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
        let mut inner = [0u8; 32];
        g.fill_bytes(&mut inner[16..]);

        Quickcheck(U256::from(&inner))
    }
}
