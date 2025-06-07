mod pipeline;

use std::sync::{Arc, Mutex};

pub use pipeline::PipelinesRepository;

pub struct Repositories {
    pub pipelines: Arc<Mutex<dyn PipelinesRepository>>,
}

impl Repositories {
    pub fn build(database_url: &str) -> Result<Repositories, String> {
        let pipelines = pipeline::implementation::PipelinesRepository::create(database_url)?;
        let pipelines = Arc::new(Mutex::new(pipelines));

        Ok(Repositories { pipelines })
    }
}
