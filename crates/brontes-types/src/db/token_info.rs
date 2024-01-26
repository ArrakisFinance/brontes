use alloy_rlp::{Decodable, Encodable};
use bytes::BufMut;
use reth_db::{
    table::{Compress, Decompress},
    DatabaseError,
};
use rkyv::Deserialize;
use sorella_db_databases::{clickhouse, clickhouse::Row};

#[derive(
    Debug, Clone, Row, serde::Serialize, rkyv::Serialize, rkyv::Deserialize, rkyv::Archive,
)]
pub struct TokenInfo {
    pub decimals: u8,
    pub symbol:   String,
}
impl TokenInfo {
    pub fn new(decimals: u8, symbol: String) -> Self {
        Self { symbol, decimals }
    }
}

impl<'de> serde::Deserialize<'de> for TokenInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let val: (u8, String) = serde::Deserialize::deserialize(deserializer)?;

        Ok(Self { decimals: val.0, symbol: val.1 })
    }
}

impl Encodable for TokenInfo {
    fn encode(&self, out: &mut dyn BufMut) {
        let encoded = rkyv::to_bytes::<_, 256>(self).unwrap();

        out.put_slice(&encoded)
    }
}

impl Decodable for TokenInfo {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let archived: &ArchivedTokenInfo = unsafe { rkyv::archived_root::<Self>(buf) };

        let this = archived.deserialize(&mut rkyv::Infallible).unwrap();

        Ok(this)
    }
}

impl Compress for TokenInfo {
    type Compressed = Vec<u8>;

    fn compress_to_buf<B: reth_primitives::bytes::BufMut + AsMut<[u8]>>(self, buf: &mut B) {
        let mut encoded = Vec::new();
        self.encode(&mut encoded);
        let encoded_compressed = zstd::encode_all(&*encoded, 0).unwrap();

        buf.put_slice(&encoded_compressed);
    }
}

impl Decompress for TokenInfo {
    fn decompress<B: AsRef<[u8]>>(value: B) -> Result<Self, reth_db::DatabaseError> {
        let binding = value.as_ref().to_vec();

        let encoded_decompressed = zstd::decode_all(&*binding).unwrap();
        let buf = &mut encoded_decompressed.as_slice();

        TokenInfo::decode(buf).map_err(|_| DatabaseError::Decode)
    }
}
