use std::{fmt::Display, str::FromStr};

use diesel::{
    backend::Backend,
    deserialize::{self, FromSql, FromSqlRow},
    serialize::{self, ToSql},
    sql_types::{Integer, VarChar},
    AsExpression,
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

#[derive(Serialize, Deserialize, Debug, AsExpression, FromSqlRow)]
#[diesel(sql_type = Integer)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PipelineId(pub i32);

impl<DB> ToSql<VarChar, DB> for PipelineStatus
where
    DB: Backend,
    str: ToSql<VarChar, DB>,
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

impl<DB> FromSql<VarChar, DB> for PipelineStatus
where
    DB: Backend,
    *const str: FromSql<diesel::sql_types::VarChar, DB>,
{
    fn from_sql(bytes: <DB as Backend>::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        let string = <String as deserialize::FromSql<VarChar, DB>>::from_sql(bytes)?;
        string
            .parse()
            .map_err(|_| panic!("Could not parse pipeline status {string}"))
    }
}

impl<DB> ToSql<Integer, DB> for PipelineId
where
    DB: Backend,
    i32: serialize::ToSql<Integer, DB>,
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, DB>,
    ) -> diesel::serialize::Result {
        self.0.to_sql(out)
    }
}

impl<DB> FromSql<Integer, DB> for PipelineId
where
    DB: Backend,
    i32: deserialize::FromSql<Integer, DB>,
{
    fn from_sql(bytes: <DB as Backend>::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        let result = <i32 as deserialize::FromSql<Integer, DB>>::from_sql(bytes);
        result.map(|id| PipelineId(id))
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
#[derive(Serialize, Deserialize, PartialEq, Debug, AsExpression, FromSqlRow)]
#[diesel(sql_type = VarChar)]
pub enum PipelineStatus {
    Pending,
    Running,
    Passed,
    Failed,
}

impl FromStr for PipelineStatus {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "pending" => Ok(PipelineStatus::Pending),
            "running" => Ok(PipelineStatus::Running),
            "failed" => Ok(PipelineStatus::Failed),
            "passed" => Ok(PipelineStatus::Passed),
            _ => Err(()),
        }
    }
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
    pub fn new(i: i32) -> Self {
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
