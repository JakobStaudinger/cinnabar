if [ -n "$NETRC_CONTENT" ]; then
  install -m 600 /dev/null $HOME/.netrc
  echo "$NETRC_CONTENT" > $HOME/.netrc
fi

set -e
