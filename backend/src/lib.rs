use domain::pipeline::{Pipeline, PipelineConfiguration, PipelineId, StepConfiguration};

mod domain;
mod runner;

pub async fn main() {
    let runner = runner::PipelineRunner::new();
    runner
        .run_pipeline(&Pipeline {
            id: PipelineId::new(1),
            configuration: PipelineConfiguration {
                name: "Test".into(),
                steps: vec![StepConfiguration {
                    name: "Step 1".into(),
                    image: "hello-world".into(),
                    commands: None,
                }],
            },
        })
        .await;
}
