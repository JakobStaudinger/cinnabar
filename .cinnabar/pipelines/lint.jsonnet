{
  name: 'Lint',
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
      name: 'Lint',
      image: 'rust:1.82.0-alpine',
      commands: [
        'apk add musl-dev',
        'rustup component add clippy',
        'cargo clippy',
      ],
      cache: [
        'target',
      ],
    },
  ],
}
