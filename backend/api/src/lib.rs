//! Definition of the GraphQL API.

use deadpool_postgres::Pool;

use tobira_util::prelude::*;
use crate::{
    mutation::Mutation,
    query::Query,
    subscription::Subscription,
};

pub mod mutation;
pub mod query;
pub mod subscription;

mod model;
mod id;
mod util;

pub(crate) use id::{Id, Key};


/// Creates and returns the API root node.
pub fn root_node() -> RootNode<'static> {
    RootNode::new(Query(&()), Mutation::new(), Subscription::new())
}

/// Type of our API root node.
pub type RootNode<'ctx> = juniper::RootNode<'static, Query<'ctx>, Mutation<'ctx>, Subscription<'ctx>>;


/// The context that is accessible to every resolver in our API.
pub struct Context<'a> {
    db: Pool,
    realm_tree: model::realm::Tree<'a>,
    dummy: &'a (),
}

impl<'a> Context<'a> {
    pub async fn new(db: Pool) -> Result<Self> {
        let realm_tree = model::realm::Tree::load(&db).await?;
        Ok(Self {
            db,
            realm_tree,
            dummy: &(),
        })
    }
}

impl<'a> juniper::Context for Context<'a> {}
