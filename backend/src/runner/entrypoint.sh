apk add git openssh
mkdir -p ~/.ssh
install -m 600 /dev/null ~/.ssh/known_hosts
ssh-keyscan github.com >> ~/.ssh/known_hosts

if [ -n "$NETRC" ]; then
  install -m 600 /dev/null $HOME/.netrc
  echo "$NETRC" > $HOME/.netrc
fi

set -e
