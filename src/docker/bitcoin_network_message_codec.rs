use bytes::{BufMut, BytesMut};
use rust_bitcoin::{consensus::encode, network::message::RawNetworkMessage};
use std::io;

pub struct RawNetworkMessageCodec;

impl tokio::codec::Decoder for RawNetworkMessageCodec {
    type Item = RawNetworkMessage;
    type Error = encode::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match encode::deserialize_partial::<RawNetworkMessage>(&src) {
            // In this case we just have an incomplete data, so we need to read more
            Err(encode::Error::Io(ref err)) if err.kind() == io::ErrorKind::UnexpectedEof => {
                Ok(None)
            }
            Err(err) => Err(err),
            // We have successfully read from the buffer
            Ok((message, bytes_read)) => {
                src.advance(bytes_read);
                Ok(Some(message))
            }
        }
    }
}

impl tokio::codec::Encoder for RawNetworkMessageCodec {
    type Item = RawNetworkMessage;
    type Error = encode::Error;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let bytes = encode::serialize(&item);

        dst.reserve(bytes.len());
        dst.put(bytes);

        Ok(())
    }
}
