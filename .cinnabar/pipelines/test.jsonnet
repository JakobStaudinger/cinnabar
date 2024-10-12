{
  name: 'Test',
  trigger: [
    {
      event: 'push',
      branch: 'main',
    },
    {
      event: 'pull_request',
      target: 'main',
    },
  ],
  steps: [
    {
      name: 'clone',
      image: 'alpine',
      commands: [
        'apk add --no-cache git',
        'git init',
        'git remote add origin https://github.com/JakobStaudinger/rust-ci.git',
        'git fetch origin +refs/heads/main',
        'git checkout main',
      ],
    },
    {
      name: 'Test',
      image: 'rust:1.78.0-alpine',
      commands: [
        'apk add musl-dev sqlite-dev',
        'cargo test',
      ],
      cache: [
        'target',
      ],
    },
  ],
}
