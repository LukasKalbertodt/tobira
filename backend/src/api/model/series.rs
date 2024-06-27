use chrono::{DateTime, Utc};
use juniper::{graphql_object, GraphQLObject, GraphQLInputObject};
use postgres_types::ToSql;

use crate::{
    api::{
        Context,
        err::ApiResult,
        Id,
        model::{
            realm::Realm,
            event::{AuthorizedEvent, EventSortOrder}
        },
        Node,
    },
    db::{types::{ExtraMetadata, Key, SeriesState as State}, util::impl_from_db},
    prelude::*,
};


pub(crate) struct Series {
    pub(crate) key: Key,
    opencast_id: String,
    synced_data: Option<SyncedSeriesData>,
    title: String,
    created: Option<DateTime<Utc>>,
    metadata: Option<ExtraMetadata>,
}

#[derive(GraphQLObject)]
struct SyncedSeriesData {
    description: Option<String>,
}

impl_from_db!(
    Series,
    select: {
        series.{ id, opencast_id, state, title, description, created, metadata },
    },
    |row| {
        Series {
            key: row.id(),
            opencast_id: row.opencast_id(),
            title: row.title(),
            created: row.created(),
            metadata: row.metadata(),
            synced_data: (State::Ready == row.state()).then(
                || SyncedSeriesData {
                    description: row.description(),
                },
            ),
        }
    },
);

impl Series {
    pub(crate) async fn load_by_id(id: Id, context: &Context) -> ApiResult<Option<Self>> {
        if let Some(key) = id.key_for(Id::SERIES_KIND) {
            Self::load_by_key(key, context).await
        } else {
            Ok(None)
        }
    }

    pub(crate) async fn load_by_key(key: Key, context: &Context) -> ApiResult<Option<Self>> {
        Self::load_by_any_id("id", &key, context).await
    }

    pub(crate) async fn load_by_opencast_id(id: String, context: &Context) -> ApiResult<Option<Self>> {
        Self::load_by_any_id("opencast_id", &id, context).await
    }

    async fn load_by_any_id(
        col: &str,
        id: &(dyn ToSql + Sync),
        context: &Context,
    ) -> ApiResult<Option<Self>> {
        let selection = Self::select();
        let query = format!("select {selection} from series where {col} = $1");
        context.db
            .query_opt(&query, &[id])
            .await?
            .map(|row| Self::from_row_start(&row))
            .pipe(Ok)
    }

    pub(crate) async fn create(series: NewSeries, context: &Context) -> ApiResult<Self> {
        let selection = Self::select();
        let query = format!(
            "insert into series (opencast_id, title, state, updated) \
                values ($1, $2, 'waiting', '-infinity') \
                returning {selection}",
        );
        context.db(context.require_tobira_admin()?)
            .query_one(&query, &[&series.opencast_id, &series.title])
            .await?
            .pipe(|row| Self::from_row_start(&row))
            .pipe(Ok)
    }
}

/// Represents an Opencast series.
#[graphql_object(Context = Context)]
impl Series {
    fn id(&self) -> Id {
        Node::id(self)
    }

    fn opencast_id(&self) -> &str {
        &self.opencast_id
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn created(&self) -> &Option<DateTime<Utc>> {
        &self.created
    }

    fn metadata(&self) -> &Option<ExtraMetadata> {
        &self.metadata
    }

    fn synced_data(&self) -> &Option<SyncedSeriesData> {
        &self.synced_data
    }

    async fn host_realms(&self, context: &Context) -> ApiResult<Vec<Realm>> {
        let selection = Realm::select();
        let query = format!("\
            select {selection} \
            from realms \
            where exists ( \
                select 1 as contains \
                from blocks \
                where realm = realms.id \
                and type = 'series' \
                and series = $1 \
            ) \
        ");
        let id = self.id().key_for(Id::SERIES_KIND).unwrap();
        context.db.query_mapped(&query, dbargs![&id], |row| Realm::from_row_start(&row))
            .await?
            .pipe(Ok)
    }

    #[graphql(arguments(order(default = Default::default())))]
    async fn events(&self, order: EventSortOrder, context: &Context) -> ApiResult<Vec<AuthorizedEvent>> {
        AuthorizedEvent::load_for_series(self.key, order, context).await
    }

    /// Returns `true` if the realm has a series block with this series.
    /// Otherwise, `false` is returned.
    pub(crate) async fn is_referenced_by_realm(&self, path: String, context: &Context) -> ApiResult<bool> {
        let query = "select exists(\
            select 1 \
            from blocks \
            join realms on blocks.realm = realms.id \
            where realms.full_path = $1 and blocks.series = $2 \
        )";
        context.db.query_one(&query, &[&path.trim_end_matches('/'), &self.key])
            .await?
            .get::<_, bool>(0)
            .pipe(Ok)
    }
}

impl Node for Series {
    fn id(&self) -> Id {
        Id::series(self.key)
    }
}


#[derive(GraphQLInputObject)]
pub(crate) struct NewSeries {
    opencast_id: String,
    title: String,
    // TODO In the future this `struct` can be extended with additional
    // (potentially optional) fields. For now we only need these.
    // Since `mountSeries` feels even more like a private API
    // in some way, and since passing stuff like metadata isn't trivial either
    // I think it's okay to leave it at that for now.
}
