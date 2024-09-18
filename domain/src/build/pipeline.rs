use std::fmt::Display;

use diesel::{
    backend::Backend,
    prelude::{Insertable, Queryable},
    serialize::ToSql,
    sql_types::{Integer, Text},
    AsExpression, Connection, Selectable, SelectableHelper, SqliteConnection,
};
use serde::{Deserialize, Serialize};

use super::{docker_image_reference::DockerImageReference, trigger::TriggerConfiguration};

#[derive(Serialize, Deserialize)]
pub struct PipelineConfiguration {
    pub name: String,
    pub trigger: Vec<TriggerConfiguration>,
    pub steps: Vec<StepConfiguration>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StepConfiguration {
    pub name: String,
    pub image: DockerImageReference,
    pub commands: Option<Vec<String>>,
    pub cache: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize)]
pub struct Pipeline {
    pub id: PipelineId,
    pub configuration: PipelineConfiguration,
    pub steps: Vec<Step>,
    pub status: PipelineStatus,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::pipelines)]
struct RawPipeline {
    pub id: PipelineId,
    pub status: PipelineStatus,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::pipelines)]
pub struct NewPipeline {
    pub status: PipelineStatus,
}

#[derive(Serialize, Deserialize, Debug, AsExpression)]
#[diesel(sql_type = Integer)]
pub struct PipelineId(pub usize);

// impl<DB> ToSql<Integer, DB> for PipelineId
// where
//     DB: Backend,
//     usize: ToSql<diesel::sql_types::Integer, DB>,
// {
//     fn to_sql<'b>(
//         &'b self,
//         out: &mut diesel::serialize::Output<'b, '_, DB>,
//     ) -> diesel::serialize::Result {
//         self.0.to_sql(out)
//     }
// }

impl<DB> ToSql<Text, DB> for PipelineStatus
where
    DB: Backend,
    str: ToSql<Text, DB>,
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, DB>,
    ) -> diesel::serialize::Result {
        match self {
            PipelineStatus::Pending => "pending".to_sql(out),
            PipelineStatus::Running => "running".to_sql(out),
            PipelineStatus::Failed => "failed".to_sql(out),
            PipelineStatus::Passed => "passed".to_sql(out),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Step {
    pub id: StepId,
    pub configuration: StepConfiguration,
    pub status: PipelineStatus,
}

#[derive(Serialize, Deserialize)]
pub struct StepId(usize);

#[repr(i32)]
#[derive(Serialize, Deserialize, PartialEq, AsExpression, Debug)]
#[diesel(sql_type = Integer)]
pub enum PipelineStatus {
    Pending,
    Running,
    Passed,
    Failed,
}

impl Pipeline {
    pub fn new(id: PipelineId, configuration: PipelineConfiguration) -> Self {
        let steps = configuration
            .steps
            .iter()
            .enumerate()
            .map(|(id, step_configuration)| {
                Step::new(StepId::new(id + 1), step_configuration.clone())
            })
            .collect();

        Self {
            id,
            configuration,
            steps,
            status: PipelineStatus::Pending,
        }
    }
}

impl PipelineId {
    pub fn new(i: usize) -> Self {
        Self(i)
    }
}

impl Display for PipelineId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Step {
    pub fn new(id: StepId, configuration: StepConfiguration) -> Self {
        Self {
            id,
            configuration,
            status: PipelineStatus::Pending,
        }
    }
}

impl StepId {
    pub fn new(i: usize) -> Self {
        Self(i)
    }
}

impl Display for StepId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub trait PipelinesRepository {
    fn create_new(&mut self) -> Result<PipelineId, ()>;
}

struct PipelinesRepositoryImpl<'a> {
    pub connection: &'a mut SqliteConnection,
}

impl<'a> PipelinesRepository for PipelinesRepositoryImpl<'a> {
    fn create_new(&mut self) -> Result<PipelineId, ()> {
        use crate::schema::pipelines;
        use diesel::prelude::*;

        let result = diesel::insert_into(pipelines::table)
            .values(NewPipeline {
                status: PipelineStatus::Pending,
            })
            .returning(RawPipeline::as_returning())
            .get_result(self.connection)
            .map_err(|_| ())?;
    }
}

pub fn create_pipeline(database_url: &str) {
    let connection = SqliteConnection::establish(database_url).unwrap();
    let mut repository = PipelinesRepositoryImpl { connection };
    repository.create_new();
}
