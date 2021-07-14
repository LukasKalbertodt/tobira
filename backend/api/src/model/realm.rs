use deadpool_postgres::Pool;
use futures::stream::TryStreamExt;
use juniper::{FieldResult, graphql_object};
use std::collections::HashMap;

use tobira_util::{
    prelude::*,
    db::NO_PARAMS,
};
use crate::{
    Context, Id, Key,
    model::block::BlockValue,
    util::RowExt,
};


pub(crate) struct Realm<'ctx> {
    key: Key,
    name: String,
    parent_key: Option<Key>,
    path_segment: String,
    child_keys: Vec<Key>,
    dummy: &'ctx (),
}

#[graphql_object(Context = Context<'ctx>)]
impl<'ctx> Realm<'ctx> {
    fn id(&self) -> Id {
        Id::realm(self.key)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn path(&self, context: &Context<'ctx>) -> String {
        Tree::path(self, &context.realm_tree.realms)
    }

    fn parent(&self, context: &Context<'ctx>) -> Option<&Realm> {
        self.parent_key.map(|parent_key| &context.realm_tree.realms[&parent_key])
    }

    fn parents(&self, context: &Context<'ctx>) -> Vec<&Realm> {
        let mut parents = Tree::walk_up(self, &context.realm_tree.realms)
            .skip(1)
            .collect::<Vec<_>>();
        parents.reverse();
        parents
    }

    fn children(&self, context: &Context<'ctx>) -> Vec<&Realm> {
        self.child_keys.iter()
            .map(|child| &context.realm_tree.realms[&child])
            .collect()
    }

    /// Returns the (content) blocks of this realm.
    async fn blocks(&self, context: &Context<'ctx>) -> FieldResult<Vec<BlockValue>> {
        // TODO: this method can very easily lead to an N+1 query problem.
        // However, it is unlikely that we ever have that problem: the frontend
        // will only show one realm at a time, so the query will also only
        // request the blocks of one realm.
        BlockValue::load_for_realm(self.key, context).await
    }
}

pub(crate) struct Tree<'ctx> {
    pub(crate) realms: HashMap<Key, Realm<'ctx>>,
    from_path: HashMap<String, Key>,
}

impl<'ctx> Tree<'ctx> {
    pub(crate) async fn load(db: &Pool) -> Result<Self> {
        debug!("Loading realms from database");

        // We store the nodes of the realm tree in a hash map
        // accessible by the database ID
        let mut realms = db.get()
            .await?
            .query_raw(
                "select id, name, parent, path_segment from realms",
                NO_PARAMS,
            )
            .await?
            .map_ok(|row| {
                let key = row.get_key(0);
                Realm {
                    key,
                    name: row.get(1),
                    parent_key: if key == 0 { None } else { Some(row.get_key(2)) },
                    path_segment: row.get(3),
                    child_keys: vec![],
                }
            })
            .map_ok(|realm| (realm.key, realm))
            .try_collect::<HashMap<_, _>>()
            .await?;

        // With this, and the `parent` member of the `Realm`,
        // we already have quick access to the data of a realm's parent.
        // To also get to the children quickly we maintain a corresponding list
        // for each realm
        let keys = realms.values()
            .filter_map(|realm| {
                realm.parent_key.map(|parent_key| (realm.key, parent_key))
            })
            .collect::<Vec<_>>();
        for (key, parent_key) in keys {
            let parent = realms.get_mut(&parent_key)
                .with_context(|| format!("invalid parent {} of {}", parent_key, key))?;
            parent.child_keys.push(key);
        }

        // After this point, we should know the tree structure to be valid.
        // That is, we can now safely panic if we can't find things in our maps/lists;
        // that's totally a bug in this code, then, not an inconsistency in the db.

        // We also need a map from the full path to the proper realm.
        let from_path = realms.iter()
            .map(|(key, realm)| (Tree::path(realm, &realms), *key))
            .collect::<HashMap<_, _>>();

        debug!("Loaded {} realms from the database", realms.len());

        Ok(Tree { realms, from_path })
    }

    fn walk_up<'a>(realm: &'a Realm<'ctx>, realms: &'a HashMap<Key, Realm<'ctx>>) -> impl Iterator<Item = &'a Realm<'ctx>> {
        std::iter::successors(
            Some(realm),
            move |child| child.parent_key.map(|parent_key| &realms[&parent_key])
        )
    }

    fn path(realm: &Realm<'ctx>, realms: &HashMap<Key, Realm<'ctx>>) -> String {
        let mut segments = Tree::walk_up(realm, realms)
            .map(|realm| &*realm.path_segment)
            .collect::<Vec<_>>();
        segments.reverse();
        segments.join("/")
    }

    pub(crate) fn get_node(&self, id: &Id) -> Option<&Realm<'ctx>> {
        self.realms.get(&id.key_for(Id::REALM_KIND)?)
    }

    pub(crate) fn root(&self) -> &Realm<'ctx> {
        &self.realms[&0]
    }

    pub(crate) fn from_path(&self, path: &str) -> Option<&Realm<'ctx>> {
        // We accept path with and without a trailing slash.
        let path = if path.ends_with('/') { &path[..path.len() - 1] } else { path };
        self.from_path.get(path).map(|key| &self.realms[key])
    }
}
