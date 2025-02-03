use crate::bincode_format::Bincode;
use core::time::Duration;
use nodo::{
    codelet::{CodeletInstance, ScheduleBuilder},
    prelude::*,
};
use nodo_core::EyreResult;
use nodo_std::{Serializer, SerializerConfig, TopicJoin, TopicJoinConfig};
use serde::{Deserialize, Serialize};

mod bincode_format;
mod r#pub;
mod snappy_bincode_format;
mod sub;

pub use bincode_format::*;
pub use r#pub::*;
pub use snappy_bincode_format::*;
pub use sub::*;

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

/// Helper to simplify publishing serialized messages from multiple channels on the same socket
pub struct Publisher {
    tag: String,
    join: CodeletInstance<TopicJoin<Vec<u8>>>,
    nng_pub: CodeletInstance<NngPub>,
    schedule_builder: ScheduleBuilder,
}

impl Publisher {
    pub fn new(tag: &str, address: &str) -> Self {
        let mut join = TopicJoin::instantiate(format!("{tag}_join"), TopicJoinConfig::default());
        let mut nng_pub = NngPub::instantiate(
            format!("{tag}_nng_pub"),
            NngPubConfig {
                address: address.to_string(),
                queue_size: 24,
                enable_statistics: false,
            },
        );
        join.tx.connect(&mut nng_pub.rx).unwrap(); // SAFETY errors guaranteed to not happen
        Self {
            tag: tag.to_string(),
            join,
            nng_pub,
            schedule_builder: nodo::codelet::ScheduleBuilder::new()
                .with_name("vis")
                .with_period(Duration::from_millis(10)),
        }
    }

    pub fn schedule_builder_mut(&mut self) -> &mut ScheduleBuilder {
        &mut self.schedule_builder
    }

    pub fn publish<T>(&mut self, topic: &str, tx: &mut DoubleBufferTx<Message<T>>) -> EyreResult<()>
    where
        T: Clone + Send + Sync + Serialize + for<'a> Deserialize<'a> + 'static,
    {
        let mut ser = Serializer::new(Bincode::default()).into_instance(
            format!("{}_ser_{topic}", self.tag),
            SerializerConfig::default(),
        );

        tx.connect(&mut ser.rx)?;
        ser.tx.connect(&mut self.join.rx.add(topic.into()))?;

        self.schedule_builder.append(ser);

        Ok(())
    }

    pub fn into_sequence(self) -> Sequence {
        Sequence::new().with(self.join).with(self.nng_pub)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Bincode, NngPub, NngPubConfig, NngSub, NngSubConfig};
    use core::time::Duration;
    use nodo::prelude::*;
    use nodo_core::WithTopic;
    use nodo_runtime::Runtime;
    use nodo_std::{
        Deserializer, DeserializerConfig, Log, Pipe, PipeConfig, Serializer, SerializerConfig,
        Sink, Source,
    };
    use serde::{Deserialize, Serialize};
    use std::sync::{Arc, RwLock};

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
        .into_instance("add_topic", PipeConfig::Dynamic);

        let mut alice = NngPub::instantiate(
            "alice",
            NngPubConfig {
                address: ADDRESS.to_string(),
                queue_size: 10,
                enable_statistics: false,
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
                .into_instance("add_topic", PipeConfig::Dynamic);

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
                SUCCESS
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
                .into(),
        );

        rt.spin();

        assert_eq!(*rx_counter.read().unwrap(), MESSAGE_COUNT);
    }
}
