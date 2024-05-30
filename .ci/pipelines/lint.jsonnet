{
  name: 'Lint',
  trigger: [
    {
      event: 'push',
      branch: 'main',
    },
  ],
  steps: [
    {
      name: 'clone',
      image: 'alpine',
      commands: [
        'apk add git',
        'git init',
        'git remote add origin https://github.com/JakobStaudinger/rust-ci.git',
        'git fetch origin +refs/heads/main',
        'git checkout main',
      ],
    },
    {
      name: 'Lint',
      image: 'rust:1.78.0-alpine',
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
