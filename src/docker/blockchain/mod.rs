use crate::docker::Image;
use futures::Future;

pub mod bitcoin;
pub mod ethereum;

pub trait BlockchainImage: Image {
    type Address;
    type Amount;
    type TxId;
    type ClientError;

    fn fund(
        &self,
        address: Self::Address,
        value: Self::Amount,
    ) -> Box<dyn Future<Item = Self::TxId, Error = Self::ClientError> + Send + Sync>;
}
