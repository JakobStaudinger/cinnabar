use domain::{Pipeline, PipelineId};

mod domain;
mod runner;

pub async fn main() {
    let configuration = serde_json::from_str(
        std::fs::read_to_string("assets/test-pipeline.json")
            .unwrap()
            .as_str(),
    )
    .unwrap();

    let pipeline = Pipeline::new(PipelineId::new(1), configuration);

    let runner = runner::PipelineRunner::new();
    runner.run_pipeline(&pipeline).await;
}
