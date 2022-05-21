use actix::prelude::*;
use std::sync::Arc;
use traits::{Blockchain, StateDB};
use types::block::Block;

#[derive(Message)]
#[rtype(result = "()")]
struct ProcessBlockMsg {
    block: Block,
}

struct StateProcessor {
    statedb: Arc<dyn StateDB>,
    blockchain: Arc<dyn Blockchain>,
}

impl Actor for StateProcessor {
    type Context = Context<Self>;
}

impl Handler<ProcessBlockMsg> for StateProcessor {
    type Result = ();

    fn handle(&mut self, msg: ProcessBlockMsg, ctx: &mut Self::Context) -> Self::Result {
        let block = msg.block;
        let current_head = self.blockchain.current_header();
        todo!()
    }
}
