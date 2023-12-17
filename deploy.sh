#!/bin/sh

rm -r public/
hugo
rsync -e ssh -rv public/ shetaye@100.109.214.114:/srv/shetaye.me/
