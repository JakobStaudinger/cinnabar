if [ -n "$NETRC" ]; then
  install -m 600 /dev/null $HOME/.netrc
  echo "$NETRC" > $HOME/.netrc
fi

set -e
