use nodo_core::{BinaryFormat, Schema};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

/// Serializes with bincode and compresses with snappy
pub struct SnappyBincode<T> {
    enc_buf: Vec<u8>,
    marker: PhantomData<T>,
}

impl<T> Default for SnappyBincode<T> {
    fn default() -> Self {
        Self {
            enc_buf: Vec::with_capacity(1024),
            marker: PhantomData,
        }
    }
}

impl<T> BinaryFormat<T> for SnappyBincode<T>
where
    T: Serialize + for<'a> Deserialize<'a>,
{
    fn schema(&self) -> Schema {
        Schema {
            name: core::any::type_name::<T>().to_string(),
            encoding: String::from("bincode"),
        }
    }

    fn serialize(&mut self, data: &T) -> eyre::Result<Vec<u8>> {
        self.enc_buf.clear();
        let snap_enc = snap::write::FrameEncoder::new(&mut self.enc_buf);
        bincode::serialize_into(snap_enc, data)?;
        Ok(self.enc_buf.clone())
    }

    fn deserialize(&mut self, buffer: &[u8]) -> eyre::Result<T> {
        let dec = snap::read::FrameDecoder::new(buffer);
        let value = bincode::deserialize_from(dec)?;
        Ok(value)
    }
}
