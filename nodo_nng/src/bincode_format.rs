use core::marker::PhantomData;
use nodo_core::{BinaryFormat, Schema};
use serde::{Deserialize, Serialize};

pub struct Bincode<T>(PhantomData<T>);

impl<T> Default for Bincode<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T> BinaryFormat<T> for Bincode<T>
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
        Ok(bincode::serialize(data)?)
    }

    fn deserialize(&mut self, buffer: &[u8]) -> eyre::Result<T> {
        Ok(bincode::deserialize(&buffer)?)
    }
}
