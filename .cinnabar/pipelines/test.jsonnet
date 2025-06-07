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
        'git remote add origin https://github.com/JakobStaudinger/cinnabar.git',
        'git fetch origin +refs/heads/main',
        'git checkout main',
      ],
    },
    {
      name: 'Test',
      image: 'rust:1.87.0-alpine',
      commands: [
        'apk add musl-dev',
        'cargo test',
      ],
      cache: [
        'target',
      ],
    },
  ],
}
