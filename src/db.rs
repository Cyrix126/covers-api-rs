use chrono::NaiveDateTime;
use deadpool_diesel::mysql::Pool;
use diesel::{
    deserialize::Queryable,
    prelude::Insertable,
    query_builder::{AsChangeset, QueryId},
    Selectable,
};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
#[derive(Queryable, Selectable, Insertable, AsChangeset, QueryId)]
#[diesel(table_name = crate::schema::covers)]
#[diesel(check_for_backend(diesel::mysql::Mysql))]
pub struct Cover {
    pub id: u32,
    pub last_try: NaiveDateTime,
    pub provider: Option<u8>,
}

const MIGRATIONS: EmbeddedMigrations = embed_migrations!();
pub async fn run_migrations(pool: &Pool) -> anyhow::Result<()> {
    let conn = pool.get().await?;
    conn.interact(|conn| conn.run_pending_migrations(MIGRATIONS).map(|_| ()).unwrap())
        .await
        .unwrap();
    Ok(())
}
