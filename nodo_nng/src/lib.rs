mod r#pub;
mod sub;

pub use r#pub::*;
pub use sub::*;

use core::marker::PhantomData;
use nodo::prelude::*;
use nodo_core::BinaryFormat;
use nodo_core::EyreResult;
use nodo_core::RecorderChannelId;
use nodo_core::Schema;
use nodo_core::SerializedValue;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct NngPubSubHeader {
    pub magic: u64,
    pub seq: u64,
    pub stamp: Stamp,
    pub channel_id: RecorderChannelId,
    pub payload_checksum: u32,
}

impl NngPubSubHeader {
    pub const MAGIC: u64 = 0x90D0ABCDABCD90D0;
    pub const CRC: crc::Crc<u32> = crc::Crc::<u32>::new(&crc::CRC_32_AUTOSAR);
    pub const BINCODE_SIZE: usize = 46;
}

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

    fn serialize(&self, data: &T) -> EyreResult<Vec<u8>> {
        Ok(bincode::serialize(data)?)
    }

    fn deserialize(&self, buffer: Vec<u8>) -> EyreResult<T> {
        Ok(bincode::deserialize(&buffer)?)
    }
}

#[cfg(test)]
mod tests {
    use crate::Bincode;
    use crate::NngPub;
    use crate::NngPubConfig;
    use crate::NngSub;
    use crate::NngSubConfig;
    use core::time::Duration;
    use nodo::prelude::*;
    use nodo::runtime::Runtime;
    use nodo::runtime::RuntimeControl;
    use nodo_core::RecorderChannelId;
    use nodo_std::CallbackRx;
    use nodo_std::CallbackTx;
    use nodo_std::Deserializer;
    use nodo_std::Log;
    use nodo_std::Serializer;
    use serde::Deserialize;
    use serde::Serialize;
    use std::sync::Arc;
    use std::sync::RwLock;

    #[test]
    fn test_pub_sub() {
        env_logger::init();

        #[derive(Debug, Clone, Serialize, Deserialize)]
        struct Foo {
            number: u32,
        }

        // const ADDRESS: &str = "ipc://Danvil/nodo/nodo_nng/test_pub_sub";
        // const ADDRESS: &str = "tcp://192.168.8.228:54327";
        const ADDRESS: &str = "tcp://127.0.0.1:7789";

        const MESSAGE_COUNT: usize = 25;

        let mut rt = Runtime::new();

        let mut tx_counter = 0;
        let mut issue = CallbackTx::new(move || {
            tx_counter += 1;
            // FIXME
            Message {
                seq: tx_counter,
                stamp: Stamp {
                    acqtime: Duration::from_millis(tx_counter).into(),
                    pubtime: Duration::from_millis(tx_counter).into(),
                },
                value: Foo {
                    number: tx_counter as u32,
                },
            }
        })
        .into_instance("issue", ());

        let mut ser =
            Serializer::new(RecorderChannelId(0), Bincode::default()).into_instance("ser", ());

        let alice_cfg = NngPubConfig {
            address: ADDRESS.to_string(),
            queue_size: 100,
        };

        let mut alice = NngPub::instantiate("alice", alice_cfg);

        let bob_cfg = NngSubConfig {
            address: ADDRESS.to_string(),
            queue_size: 100,
        };

        let mut bob = NngSub::instantiate("bob", bob_cfg);

        let mut de = Deserializer::<Foo, _>::new(Bincode::default()).into_instance("de", ());

        let mut log = Log::instantiate("log", ());

        let rx_counter = Arc::new(RwLock::new(0));
        let mut check = {
            let rx_counter = rx_counter.clone();
            let ctrl = rt.tx_control();
            CallbackRx::new(move |foo: Message<Foo>| {
                assert!(foo.value.number as usize > *rx_counter.read().unwrap());
                *rx_counter.write().unwrap() += 1;
                if *rx_counter.read().unwrap() == MESSAGE_COUNT {
                    ctrl.send(RuntimeControl::RequestStop).unwrap();
                }
            })
            .into_instance("check", ())
        };

        issue.tx.connect(&mut ser.rx).unwrap();
        ser.tx.connect(&mut alice.rx).unwrap();
        bob.tx.connect(&mut de.rx).unwrap();
        de.tx.connect(&mut log.rx).unwrap();
        de.tx.connect(&mut check.rx).unwrap();

        rt.add_codelet_schedule(
            nodo::codelet::ScheduleBuilder::new()
                .with_period(Duration::from_millis(1))
                .with(issue)
                .with(ser)
                .with(alice)
                .with(bob)
                .with(de)
                .with(log)
                .with(check)
                .finalize(),
        );

        rt.spin();

        assert_eq!(*rx_counter.read().unwrap(), MESSAGE_COUNT);
    }
}
