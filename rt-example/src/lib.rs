use crate::runtime::Runtime as Rt;

mod runtime;
mod storage_api;

struct Runtime;

impl Rt for Runtime {
    fn execute(ctx: String, raw_tx: String) -> String {
        storage_api::set(2, &vec![1, 2, 3], &vec![9, 8, 7]);
        format!("{} Hahahahahhah", raw_tx, )
    }

    fn rpc(ctx: String, raw_rpc: String) -> String {
        "boooo".into()
    }
}