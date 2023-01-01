use bytes::{Buf, BufMut};
use primitive_types::H256;
use getset::{CopyGetters, Getters, MutGetters, Setters};
use prost::DecodeError;
use prost::encoding::{DecodeContext, WireType};
use serde::{Deserialize, Serialize};

pub type Log = Vec<u8>;

#[derive(
Serialize, Deserialize, Clone, Debug, Default, Getters, Setters, MutGetters, CopyGetters,
)]
pub struct Receipt {
    app_id: u32,
    tx_hash: H256,
    logs: Vec<Log>,
    fuel_used: u64,
}


impl prost::Message for Receipt {
    fn encode_raw<B>(&self, buf: &mut B)
        where
            B: BufMut,
            Self: Sized,
    {
        let mut tag = 0;
        let mut next_tag = || {
            tag += 1;
            tag
        };
        prost::encoding::uint32::encode(next_tag(), &self.app_id, buf);
        prost::encoding::bytes::encode(next_tag(), &self.tx_hash, buf);
        prost::encoding::bytes::encode_repeated(next_tag(), &self.logs, buf);
        prost::encoding::uint64::encode(next_tag(), &self.fuel_used, buf);
    }

    fn merge_field<B>(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut B,
        ctx: DecodeContext,
    ) -> std::result::Result<(), DecodeError>
        where
            B: Buf,
            Self: Sized,
    {
        const STRUCT_NAME: &str = "BlockHeader";
        match tag {
            1 => prost::encoding::uint32::merge(wire_type, &mut self.app_id, buf, ctx).map_err(
                |mut error| {
                    error.push(STRUCT_NAME, "app_id");
                    error
                },
            ),
            2 => prost::encoding::bytes::merge(wire_type, &mut self.tx_hash, buf, ctx).map_err(
                |mut error| {
                    error.push(STRUCT_NAME, "tx_hash");
                    error
                },
            ),
            3 => prost::encoding::bytes::merge_repeated(wire_type, &mut self.logs, buf, ctx)
                .map_err(|mut error| {
                    error.push(STRUCT_NAME, "logs");
                    error
                }),
            4 => prost::encoding::uint64::merge(wire_type, &mut self.fuel_used, buf, ctx).map_err(
                |mut error| {
                    error.push(STRUCT_NAME, "fuel_used");
                    error
                },
            ),
            _ => prost::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }

    fn encoded_len(&self) -> usize {
        let mut tag = 0;
        let mut next_tag = || {
            tag += 1;
            tag
        };
        prost::encoding::uint32::encoded_len(next_tag(), &self.app_id)
            + prost::encoding::bytes::encoded_len(next_tag(), &self.tx_hash)
            + prost::encoding::bytes::encoded_len_repeated(next_tag(), &self.logs)
            + prost::encoding::uint64::encoded_len(next_tag(), &self.fuel_used)
    }

    fn clear(&mut self) {
        *self = Receipt::default()
    }
}