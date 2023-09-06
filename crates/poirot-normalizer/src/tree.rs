use ethers::types::{Address, H256};
use std::collections::HashMap;
use tracing::error;

use crate::normalized_actions::NormalizedAction;

pub struct Node<V: NormalizedAction> {
    pub inner: Vec<Node<V>>,
    pub frozen: bool,

    /// This only has values when the node is frozen
    pub subactions: Vec<V>,
    pub address: Address,
    pub data: V,
}

impl<V: NormalizedAction> Node<V> {
    pub fn is_frozen(&self) -> bool {
        self.frozen
    }

    pub fn freeze(&mut self) {
        self.subactions = self.get_all_sub_actions();
        self.frozen = true
    }

    /// The address here is the from address for the trace
    pub fn insert(&mut self, address: Address, n: Node<V>) -> bool {
        if self.frozen {
            return false
        }

        if address == self.address {
            let mut cur_stack = self.current_call_stack();
            cur_stack.pop();
            if !cur_stack.contains(&address) {
                self.inner.iter_mut().for_each(|n| n.freeze());
                self.inner.push(n);
                return true
            }
        }

        let last = self.inner.last_mut().expect("building tree went wrong");
        return last.insert(address, n)
    }

    pub fn get_all_sub_actions(&self) -> Vec<V> {
        if self.frozen {
            self.subactions.clone()
        } else {
            self.inner.iter().flat_map(|inner| inner.get_all_sub_actions()).collect()
        }
    }

    pub fn current_call_stack(&self) -> Vec<Address> {
        let Some(mut stack) = self.inner.last().map(|n| n.current_call_stack()) else {
            return vec![self.address]
        };

        stack.push(self.address);

        stack
    }

    pub fn inspect<F>(&mut self, prev: &mut Vec<Vec<V>>, call: &F)
    where
        F: Fn(&mut Vec<V>) -> bool,
    {
        let mut subactions = self.get_all_sub_actions();

        // the previous sub-action was best
        if !call(&mut subactions) {
            return
        }

        // remove parent sub-action as it was suboptimal
        prev.pop();
        // add our new best sub-action
        prev.push(subactions);

        // check if inners are better
        for inner in &mut self.inner {
            inner.inspect(prev, call)
        }
    }
}

pub struct Root<V: NormalizedAction> {
    pub head: Node<V>,
    pub tx_hash: H256,
}

impl<V: NormalizedAction> Root<V> {
    pub fn insert(&mut self, from: Address, node: Node<V>) {
        if !self.head.insert(from, node) {
            error!("failed to insert node");
        }
    }

    pub fn inspect<F>(&mut self, call: &F) -> Vec<Vec<V>>
    where
        F: Fn(&mut Vec<V>) -> bool,
    {
        let mut result = Vec::new();
        self.head.inspect(&mut result, call);

        result
    }
}

pub struct TimeTree<V: NormalizedAction> {
    pub roots: Vec<Root<V>>,
}

impl<V: NormalizedAction> TimeTree<V> {
    pub fn new(txes: usize) -> Self {
        Self { roots: Vec::with_capacity(txes) }
    }

    pub fn insert_root(&mut self, root: Root<V>) {
        self.roots.push(root);
    }

    pub fn insert_node(&mut self, from: Address, node: Node<V>) {
        self.roots.last_mut().expect("no root_nodes inserted").insert(from, node);
    }

    pub fn inspect<F>(hash: H256, call: F) -> Vec<Vec<V>>
    where
        F: Fn(&mut Vec<V>) -> bool,
    {
        todo!()
    }

    pub fn inspect_all<F>(call: F) -> HashMap<H256, Vec<Vec<V>>>
    where
        F: Fn(&mut Vec<V>) -> bool,
    {
        todo!()
    }
}
