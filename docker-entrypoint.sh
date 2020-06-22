#!/bin/sh

# IF BOOK DIR IS EMPTY
CONTENT_LENGTH=$(ls -A /book | wc -m)
if [ $CONTENT_LENGTH == "0" ]; then

    # INIT NEW BOOK
    printf 'y\n \n' | mdbook init --force /book
fi

# START SERVING BOOK
mdbook serve /book --hostname 0.0.0.0 --port 3000 --websocket-port 3001
