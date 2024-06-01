mod r#pub;
mod sub;

pub use r#pub::*;
pub use sub::*;

use core::marker::PhantomData;
use nodo::prelude::*;
use nodo_core::BinaryFormat;
use nodo_core::EyreResult;
use nodo_core::Schema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct NngPubSubHeader {
    pub magic: u64,
    pub seq: u64,
    pub stamp: Stamp,
    pub payload_checksum: u32,
}

impl NngPubSubHeader {
    pub const MAGIC: u64 = 0x90D0ABCDABCD90D0;
    pub const CRC: crc::Crc<u32> = crc::Crc::<u32>::new(&crc::CRC_32_AUTOSAR);
    pub const BINCODE_SIZE: usize = 44;
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
    use nodo_core::WithTopic;
    use nodo_std::Deserializer;
    use nodo_std::DeserializerConfig;
    use nodo_std::Log;
    use nodo_std::Pipe;
    use nodo_std::Serializer;
    use nodo_std::SerializerConfig;
    use nodo_std::Sink;
    use nodo_std::Source;
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
        let mut issue = Source::new(move || {
            tx_counter += 1;
            // FIXME
            Message {
                seq: tx_counter,
                stamp: Stamp {
                    acqtime: Duration::from_millis(1000 + tx_counter).into(),
                    pubtime: Duration::from_millis(tx_counter).into(),
                },
                value: Foo {
                    number: tx_counter as u32,
                },
            }
        })
        .into_instance("issue", ());

        let mut ser = Serializer::new(Bincode::default())
            .into_instance("ser", SerializerConfig { queue_size: 1 });

        let mut add_topic = Pipe::new(|msg: Message<Vec<u8>>| {
            msg.map(|value| WithTopic {
                topic: "test".into(),
                value,
            })
        })
        .into_instance("add_topic", ());

        let mut alice = NngPub::instantiate(
            "alice",
            NngPubConfig {
                address: ADDRESS.to_string(),
                queue_size: 10,
            },
        );

        let mut bob = NngSub::instantiate(
            "bob",
            NngSubConfig {
                address: ADDRESS.to_string(),
                queue_size: 10,
            },
        );

        let mut rmv_topic =
            Pipe::new(|msg: Message<WithTopic<Vec<u8>>>| msg.map(|WithTopic { value, .. }| value))
                .into_instance("add_topic", ());

        let mut de = Deserializer::<Foo, _>::new(Bincode::default())
            .into_instance("de", DeserializerConfig { queue_size: 1 });

        let mut log = Log::instantiate("log", ());

        let rx_counter = Arc::new(RwLock::new(0));
        let mut check = {
            let rx_counter = rx_counter.clone();
            let ctrl = rt.tx_control();
            Sink::new(move |foo: Message<Foo>| {
                assert!(foo.value.number as usize > *rx_counter.read().unwrap());
                *rx_counter.write().unwrap() += 1;
                if *rx_counter.read().unwrap() == MESSAGE_COUNT {
                    ctrl.send(RuntimeControl::RequestStop)?;
                }
                Ok(())
            })
            .into_instance("check", ())
        };

        issue.tx.connect(&mut ser.rx).unwrap();
        ser.tx.connect(&mut add_topic.rx).unwrap();
        add_topic.tx.connect(&mut alice.rx).unwrap();
        bob.tx.connect(&mut rmv_topic.rx).unwrap();
        rmv_topic.tx.connect(&mut de.rx).unwrap();
        de.tx.connect(&mut log.rx).unwrap();
        de.tx.connect(&mut check.rx).unwrap();

        rt.add_codelet_schedule(
            nodo::codelet::ScheduleBuilder::new()
                .with_period(Duration::from_millis(1))
                .with(issue)
                .with(ser)
                .with(add_topic)
                .with(alice)
                .with(bob)
                .with(rmv_topic)
                .with(de)
                .with(log)
                .with(check)
                .finalize(),
        );

        rt.spin();

        assert_eq!(*rx_counter.read().unwrap(), MESSAGE_COUNT);
    }
}
