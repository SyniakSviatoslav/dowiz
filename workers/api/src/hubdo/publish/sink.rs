//! WHERE A PUBLISH WRITES AND READS (BN2, `hubdo/publish.rs`): the R2 bucket
//! and the photo KV, live; or (tests) one recording bucket that holds both, so
//! the REAL publish code runs under `cargo test` and every put it makes is a
//! fact the tests can count.

use worker::*;

/// Where the objects go: the bucket, or (tests) a list that records every put.
pub(super) enum Sink {
    Live(Bucket),
    #[cfg(test)]
    Mem(std::rc::Rc<mem::MemBucket>),
}

impl Sink {
    pub(super) async fn put(&self, key: &str, bytes: Vec<u8>, content_type: &str, cache_control: &str) -> Result<()> {
        match self {
            Sink::Live(b) => {
                let meta = HttpMetadata {
                    content_type: Some(content_type.to_string()),
                    cache_control: Some(cache_control.to_string()),
                    ..Default::default()
                };
                b.put(key, bytes).http_metadata(meta).execute().await?;
                Ok(())
            }
            #[cfg(test)]
            Sink::Mem(m) => {
                m.puts.borrow_mut().push(mem::Put { key: key.to_string(), bytes, content_type: content_type.to_string(), cache_control: cache_control.to_string() });
                Ok(())
            }
        }
    }
}

/// Where the photos come from: KV `MEDIA` (bytes under `<name>`, type under `<name>#type`).
pub(super) enum Photos {
    Live(worker::kv::KvStore),
    #[cfg(test)]
    Mem(std::rc::Rc<mem::MemBucket>),
}

impl Photos {
    pub(super) async fn get(&self, name: &str) -> Result<Option<(Vec<u8>, String)>> {
        match self {
            Photos::Live(kv) => {
                let Some(bytes) = kv.get(name).bytes().await? else { return Ok(None) };
                let kind = kv.get(&format!("{name}#type")).text().await?.unwrap_or_else(|| "application/octet-stream".into());
                Ok(Some((bytes, kind)))
            }
            #[cfg(test)]
            Photos::Mem(m) => Ok(m.photos.borrow().get(name).cloned()),
        }
    }
}

/// The test bucket: every put recorded, and the photos KV would hold.
#[cfg(test)]
pub(super) mod mem {
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::rc::Rc;

    #[derive(Debug, Clone, PartialEq)]
    pub struct Put {
        pub key: String,
        pub bytes: Vec<u8>,
        pub content_type: String,
        pub cache_control: String,
    }

    #[derive(Default)]
    pub struct MemBucket {
        pub puts: RefCell<Vec<Put>>,
        pub photos: RefCell<BTreeMap<String, (Vec<u8>, String)>>,
    }

    impl MemBucket {
        /// The keys put since the list was last drained, in order.
        pub fn drain(&self) -> Vec<String> {
            self.puts.borrow_mut().drain(..).map(|p| p.key).collect()
        }
    }

    thread_local! {
        /// The bucket a test's object publishes to (`None`: no binding).
        pub static TEST_BUCKET: RefCell<Option<Rc<MemBucket>>> = const { RefCell::new(None) };
    }

    /// Point the object under test at a fresh bucket, and hand it back.
    pub fn bucket() -> Rc<MemBucket> {
        let b = Rc::new(MemBucket::default());
        TEST_BUCKET.with(|t| *t.borrow_mut() = Some(b.clone()));
        b
    }
    /// No binding: what the deployment is until the operator step.
    pub fn no_bucket() {
        TEST_BUCKET.with(|t| *t.borrow_mut() = None);
    }
}
