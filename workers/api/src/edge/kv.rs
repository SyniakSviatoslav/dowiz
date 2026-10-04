//! The KV store behind the media routes: see [`Kv`].

use worker::Result;

#[cfg(test)]
use super::mem;

/// `worker::kv::KvStore`, over the four calls the media routes make (W-COV C2): the Live arm is
/// the platform's builder chain, unchanged; in a test the World holds the bytes.
pub enum Kv {
    Live(worker::kv::KvStore),
    #[cfg(test)]
    Mem(std::rc::Rc<mem::World>),
}

impl Kv {
    pub async fn put_bytes(&self, key: &str, value: &[u8]) -> Result<()> {
        match self {
            Kv::Live(k) => Ok(k.put_bytes(key, value)?.execute().await?),
            #[cfg(test)]
            Kv::Mem(w) => {
                w.kv.borrow_mut().insert(key.to_string(), value.to_vec());
                Ok(())
            }
        }
    }
    pub async fn put_text(&self, key: &str, value: &str) -> Result<()> {
        match self {
            Kv::Live(k) => Ok(k.put(key, value)?.execute().await?),
            #[cfg(test)]
            Kv::Mem(w) => {
                w.kv.borrow_mut().insert(key.to_string(), value.as_bytes().to_vec());
                Ok(())
            }
        }
    }
    pub async fn get_bytes(&self, key: &str) -> Result<Option<Vec<u8>>> {
        match self {
            Kv::Live(k) => Ok(k.get(key).bytes().await?),
            #[cfg(test)]
            Kv::Mem(w) => Ok(w.kv.borrow().get(key).cloned()),
        }
    }
    pub async fn get_text(&self, key: &str) -> Result<Option<String>> {
        match self {
            Kv::Live(k) => Ok(k.get(key).text().await?),
            #[cfg(test)]
            Kv::Mem(w) => Ok(w.kv.borrow().get(key).map(|b| String::from_utf8_lossy(b).into_owned())),
        }
    }
}
