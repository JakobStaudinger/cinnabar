use diesel::prelude::*;

use crate::{PipelineId, PipelineStatus};

pub struct PipelinesRepository {
    connection: SqliteConnection,
}

impl PipelinesRepository {
    pub fn create(database_url: &str) -> Result<Self, String> {
        let connection = SqliteConnection::establish(database_url)
            .map_err(|e| format!("Could not establish database connection: {e}"))?;

        Ok(Self { connection })
    }
}

impl super::PipelinesRepository for PipelinesRepository {
    fn create_new(&mut self) -> Result<PipelineId, ()> {
        use crate::schema::pipelines;

        let pipeline = NewPipeline {
            status: PipelineStatus::Pending,
        };

        let result = diesel::insert_into(pipelines::table)
            .values(pipeline)
            .returning(RawPipeline::as_returning())
            .get_result(&mut self.connection)
            .map_err(|_| ())?;

        Ok(result.id)
    }
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::pipelines)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
struct RawPipeline {
    pub id: PipelineId,
    pub status: PipelineStatus,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::pipelines)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
struct NewPipeline {
    pub status: PipelineStatus,
}
