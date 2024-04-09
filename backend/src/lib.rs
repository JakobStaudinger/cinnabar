use domain::pipeline::{Pipeline, PipelineId};

mod domain;
mod runner;

pub async fn main() {
    let runner = runner::PipelineRunner::new();
    runner
        .run_pipeline(&Pipeline {
            id: PipelineId::new(1),
            configuration: serde_json::from_str(
                std::fs::read_to_string("assets/test-pipeline.json")
                    .unwrap()
                    .as_str(),
            )
            .unwrap(),
        })
        .await;
}
