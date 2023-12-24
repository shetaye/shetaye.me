#!/bin/sh

rm -r public/
hugo
rsync -e ssh -rv public/ shetaye@babylon:/srv/shetaye.me/
